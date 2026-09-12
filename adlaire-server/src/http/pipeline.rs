use std::{convert::Infallible, sync::Arc};

use http_body_util::BodyExt;
use hyper::{body::Incoming, Request};

use crate::{
    auth::{middleware::extract_claims, AccessLevel, Claims},
    db::{sqld_adapter::SqldAdapter, validate_db_name},
    error::AppError,
    hrana::{
        convert::{hrana_to_sql, sql_to_stmt_result},
        types::{HranaError, PipelineRequest, PipelineResponse, StreamRequest, StreamResponse, StreamResult},
    },
    http::HttpResponse,
    state::SharedState,
};

pub async fn handle(
    req: Request<Incoming>,
    state: SharedState,
    db_name: &str,
) -> Result<HttpResponse, Infallible> {
    if let Err(e) = validate_db_name(db_name) {
        return Ok(e.into_response());
    }

    let claims = match extract_claims(&req, &state).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };
    let req = match parse_json_body::<PipelineRequest>(req).await {
        Ok(r) => r,
        Err(e) => return Ok(e.into_response()),
    };
    let db = match state.db_mgr.get(db_name).await {
        Some(db) => db,
        None => return Ok(AppError::DbNotFound(db_name.to_string()).into_response()),
    };
    match execute_pipeline(&db, &claims, &req.requests, db_name).await {
        Ok(results) => Ok(crate::http::json_ok(&PipelineResponse {
            baton: None,
            base_url: None,
            results,
        })),
        Err(e) => Ok(e.into_response()),
    }
}

async fn parse_json_body<T: serde::de::DeserializeOwned>(
    req: Request<Incoming>,
) -> Result<T, AppError> {
    let bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|_| AppError::InvalidRequest)?
        .to_bytes();
    serde_json::from_slice::<T>(&bytes).map_err(|_| AppError::InvalidRequest)
}

async fn execute_pipeline(
    db:       &Arc<dyn SqldAdapter>,
    claims:   &Claims,
    requests: &[StreamRequest],
    db_name:  &str,
) -> Result<Vec<StreamResult>, AppError> {
    let mut results = Vec::with_capacity(requests.len());

    for req in requests {
        match req {
            StreamRequest::Execute { stmt } => {
                if !stmt.named_args.is_empty() {
                    results.push(StreamResult::Error {
                        error: HranaError {
                            message: "named arguments are not supported yet".into(),
                            code:    "SQLITE_ERROR".into(),
                        },
                    });
                    continue;
                }

                if is_write_stmt(&stmt.sql)
                    && claims.resolve_access(db_name) != AccessLevel::Rw
                {
                    results.push(StreamResult::Error {
                        error: HranaError {
                            message: "write not permitted".into(),
                            code:    "PERMISSION_DENIED".into(),
                        },
                    });
                    continue;
                }

                let sql_args: Result<Vec<_>, _> = stmt.args.iter().map(hrana_to_sql).collect();
                let sql_args = match sql_args {
                    Ok(a) => a,
                    Err(_) => {
                        results.push(StreamResult::Error {
                            error: HranaError {
                                message: "invalid argument value".into(),
                                code:    "SQLITE_ERROR".into(),
                            },
                        });
                        continue;
                    }
                };
                match db.execute(&stmt.sql, sql_args, stmt.want_rows).await {
                    Ok(r) => results.push(StreamResult::Ok {
                        response: StreamResponse::Execute {
                            result: sql_to_stmt_result(r),
                        },
                    }),
                    Err(AppError::Sqld(msg)) => results.push(StreamResult::Error {
                        error: HranaError {
                            message: msg.clone(),
                            code:    sqld_error_code(&msg),
                        },
                    }),
                    Err(e) => return Err(e),
                }
            }

            StreamRequest::Sequence { sql } => {
                match db.execute_batch(sql).await {
                    Ok(()) => results.push(StreamResult::Ok {
                        response: StreamResponse::Sequence,
                    }),
                    Err(AppError::Sqld(msg)) => results.push(StreamResult::Error {
                        error: HranaError {
                            message: msg.clone(),
                            code:    sqld_error_code(&msg),
                        },
                    }),
                    Err(e) => return Err(e),
                }
            }

            StreamRequest::Close => {
                results.push(StreamResult::Ok {
                    response: StreamResponse::Close,
                });
                break;
            }
        }
    }

    Ok(results)
}

fn is_write_stmt(sql: &str) -> bool {
    let upper = sql.trim_start().to_ascii_uppercase();
    matches!(
        upper.split_whitespace().next().unwrap_or(""),
        "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP"
            | "ALTER" | "REPLACE" | "PRAGMA"
    )
}

fn sqld_error_code(msg: &str) -> String {
    if msg.contains("UNIQUE constraint") {
        "SQLITE_CONSTRAINT".into()
    } else {
        "SQLITE_ERROR".into()
    }
}
