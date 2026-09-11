use std::sync::Arc;

use axum::{Json, extract::State};

use crate::{
    auth::{AccessLevel, middleware::Authenticated},
    db::sqld_adapter::SqldAdapter,
    error::AppError,
    hrana::{
        convert::{hrana_to_sql, sql_to_stmt_result},
        types::{HranaError, PipelineRequest, PipelineResponse, StreamRequest, StreamResponse, StreamResult},
    },
    state::{AppState, SharedState},
};

// ── シングル DB ハンドラ（Phase 3） ───────────────────────────────────────────

pub async fn handle(
    State(state):        State<SharedState>,
    Authenticated(claims): Authenticated,
    Json(req):           Json<PipelineRequest>,
) -> Result<Json<PipelineResponse>, AppError> {
    let db = state
        .db_mgr
        .get("default")
        .await
        .ok_or_else(|| AppError::DbNotFound("default".to_string()))?;
    let results = execute_pipeline(&db, &claims, &req.requests, "default").await?;
    Ok(Json(PipelineResponse { baton: None, base_url: None, results }))
}

// ── マルチ DB ハンドラ（Phase 6 用スタブ） ────────────────────────────────────

pub async fn handle_db(
    State(state):        State<SharedState>,
    Authenticated(claims): Authenticated,
    axum::extract::Path(db_name): axum::extract::Path<String>,
    Json(req):           Json<PipelineRequest>,
) -> Result<Json<PipelineResponse>, AppError> {
    let db = state
        .db_mgr
        .get(&db_name)
        .await
        .ok_or_else(|| AppError::DbNotFound(db_name.clone()))?;
    let results = execute_pipeline(&db, &claims, &req.requests, &db_name).await?;
    Ok(Json(PipelineResponse { baton: None, base_url: None, results }))
}

// ── パイプライン実行コア ──────────────────────────────────────────────────────

async fn execute_pipeline(
    db:       &Arc<dyn SqldAdapter>,
    claims:   &crate::auth::Claims,
    requests: &[StreamRequest],
    db_name:  &str,
) -> Result<Vec<StreamResult>, AppError> {
    let mut results = Vec::with_capacity(requests.len());

    for req in requests {
        match req {
            StreamRequest::Execute { stmt } => {
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

                let sql_args: Vec<_> = stmt.args.iter().map(hrana_to_sql).collect();
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
                for stmt_sql in split_sql_statements(sql) {
                    match db.execute(&stmt_sql, vec![], false).await {
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

// ── ヘルパー ──────────────────────────────────────────────────────────────────

fn is_write_stmt(sql: &str) -> bool {
    let upper = sql.trim_start().to_ascii_uppercase();
    matches!(
        upper.split_whitespace().next().unwrap_or(""),
        "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP"
            | "ALTER" | "REPLACE" | "PRAGMA"
    )
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn sqld_error_code(msg: &str) -> String {
    if msg.contains("UNIQUE constraint") {
        "SQLITE_CONSTRAINT".into()
    } else {
        "SQLITE_ERROR".into()
    }
}
