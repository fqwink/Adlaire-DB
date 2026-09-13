use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{body::Incoming, header, upgrade::Upgraded, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio_tungstenite::{
    tungstenite::{
        handshake::derive_accept_key,
        protocol::{frame::coding::CloseCode, CloseFrame, Role},
        Message,
    },
    WebSocketStream,
};

use crate::{
    auth::{AccessLevel, Claims},
    db::{sqld_adapter::SqldAdapter, validate_db_name, DbInfo},
    error::AppError,
    hrana::{
        convert::{hrana_to_sql, sql_to_stmt_result},
        types::{HranaError, Stmt, StmtResult},
    },
    http::HttpResponse,
    state::SharedState,
};

const MAX_FRAME_BYTES: usize = 1024 * 1024;

type Ws = WebSocketStream<TokioIo<Upgraded>>;

pub async fn handle(
    req: Request<Incoming>,
    state: SharedState,
    db_name: &str,
) -> Result<HttpResponse, Infallible> {
    if let Err(e) = validate_db_name(db_name) {
        return Ok(e.into_response());
    }
    let db = match state.db_mgr.get(db_name).await {
        Some(db) => db,
        None => return Ok(AppError::DbNotFound(db_name.to_string()).into_response()),
    };
    let db_info = match state.db_mgr.get_info(db_name).await {
        Ok(info) => info,
        Err(e) => return Ok(e.into_response()),
    };
    if !is_upgrade_request(&req) {
        return Ok(status_response(StatusCode::BAD_REQUEST));
    }
    let Some(key) = req
        .headers()
        .get(header::SEC_WEBSOCKET_KEY)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
    else {
        return Ok(status_response(StatusCode::BAD_REQUEST));
    };
    let protocol = select_protocol(&req);
    let on_upgrade = hyper::upgrade::on(req);
    let state_for_task = Arc::clone(&state);
    let db_name = db_name.to_string();
    tokio::spawn(async move {
        match on_upgrade.await {
            Ok(upgraded) => {
                let io = TokioIo::new(upgraded);
                let ws = WebSocketStream::from_raw_socket(io, Role::Server, None).await;
                if let Err(e) = run_socket(ws, state_for_task, db, db_info, db_name).await {
                    tracing::debug!(err = %e, "WebSocket session closed");
                }
            }
            Err(e) => tracing::debug!(err = %e, "WebSocket upgrade failed"),
        }
    });

    let mut builder = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header(
            header::SEC_WEBSOCKET_ACCEPT,
            derive_accept_key(key.as_bytes()),
        );
    if let Some(protocol) = protocol {
        builder = builder.header(header::SEC_WEBSOCKET_PROTOCOL, protocol);
    }
    Ok(builder.body(Full::from(Bytes::new())).unwrap())
}

fn is_upgrade_request(req: &Request<Incoming>) -> bool {
    let has_connection_upgrade = req
        .headers()
        .get(header::CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("upgrade"))
        })
        .unwrap_or(false);
    let has_upgrade_websocket = req
        .headers()
        .get(header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);
    let version_13 = req
        .headers()
        .get(header::SEC_WEBSOCKET_VERSION)
        .and_then(|v| v.to_str().ok())
        == Some("13");
    has_connection_upgrade
        && has_upgrade_websocket
        && version_13
        && req.headers().contains_key(header::SEC_WEBSOCKET_KEY)
}

fn select_protocol<B>(req: &Request<B>) -> Option<&'static str> {
    let raw = req
        .headers()
        .get(header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|v| v.to_str().ok())?;
    raw.split(',').find_map(|part| match part.trim() {
        "hrana3" => Some("hrana3"),
        "hrana2" => Some("hrana2"),
        "hrana1" => Some("hrana1"),
        _ => None,
    })
}

async fn run_socket(
    mut ws: Ws,
    state: SharedState,
    db: Arc<dyn SqldAdapter>,
    db_info: DbInfo,
    db_name: String,
) -> anyhow::Result<()> {
    let first = match ws.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()),
    };
    let text = match message_to_text(first, &mut ws).await? {
        MessageAction::Text(text) => text,
        MessageAction::Continue | MessageAction::Close => return Ok(()),
    };
    let hello = match serde_json::from_str::<ClientMessage>(&text) {
        Ok(ClientMessage::Hello { jwt }) => jwt,
        _ => {
            send_json(
                &mut ws,
                &ServerMessage::HelloError {
                    error: hrana_error(AppError::AuthInvalid),
                },
            )
            .await?;
            let _ = ws
                .close(Some(CloseFrame {
                    code: CloseCode::Protocol,
                    reason: "hello required".into(),
                }))
                .await;
            return Ok(());
        }
    };
    let claims = match authenticate_ws(&state, hello.as_deref()).await {
        Ok(claims) => claims,
        Err(e) => {
            send_json(
                &mut ws,
                &ServerMessage::HelloError {
                    error: hrana_error(e),
                },
            )
            .await?;
            let _ = ws
                .close(Some(CloseFrame {
                    code: CloseCode::Protocol,
                    reason: "auth failed".into(),
                }))
                .await;
            return Ok(());
        }
    };
    if let Err(e) = authorize_scope(&claims, &db_info) {
        send_json(
            &mut ws,
            &ServerMessage::HelloError {
                error: hrana_error(e),
            },
        )
        .await?;
        let _ = ws
            .close(Some(CloseFrame {
                code: CloseCode::Protocol,
                reason: "scope denied".into(),
            }))
            .await;
        return Ok(());
    }
    send_json(&mut ws, &ServerMessage::HelloOk).await?;

    let quota_exceeded = state
        .db_mgr
        .quota_exceeded_for_db(&db_name)
        .await
        .unwrap_or(false);
    let mut session = WsSession::new(db, db_info, claims, quota_exceeded);
    while let Some(msg) = ws.next().await {
        let msg = msg?;
        let text = match message_to_text(msg, &mut ws).await? {
            MessageAction::Text(text) => text,
            MessageAction::Continue => continue,
            MessageAction::Close => break,
        };
        let parsed = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(message) => message,
            Err(_) => {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Protocol,
                        reason: "invalid json".into(),
                    }))
                    .await;
                break;
            }
        };
        match parsed {
            ClientMessage::Hello { .. } => {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Protocol,
                        reason: "duplicate hello".into(),
                    }))
                    .await;
                break;
            }
            ClientMessage::Request {
                request_id,
                stream_id,
                body,
            } => match session.handle_request(stream_id, body).await {
                Ok(response) => {
                    send_json(
                        &mut ws,
                        &ServerMessage::ResponseOk {
                            request_id,
                            response,
                        },
                    )
                    .await?;
                }
                Err(e) => {
                    send_json(
                        &mut ws,
                        &ServerMessage::ResponseError {
                            request_id,
                            error: hrana_error(e),
                        },
                    )
                    .await?;
                }
            },
            ClientMessage::CloseStream { stream_id } => {
                session.close_stream(stream_id).await;
            }
        }
    }
    session.rollback_open_streams().await;
    Ok(())
}

enum MessageAction {
    Text(String),
    Continue,
    Close,
}

async fn message_to_text(msg: Message, ws: &mut Ws) -> anyhow::Result<MessageAction> {
    match msg {
        Message::Text(text) => {
            if text.len() > MAX_FRAME_BYTES {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Size,
                        reason: "frame too large".into(),
                    }))
                    .await;
                return Ok(MessageAction::Close);
            }
            Ok(MessageAction::Text(text))
        }
        Message::Binary(bytes) => {
            if bytes.len() > MAX_FRAME_BYTES {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Size,
                        reason: "frame too large".into(),
                    }))
                    .await;
            } else {
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Unsupported,
                        reason: "binary unsupported".into(),
                    }))
                    .await;
            }
            Ok(MessageAction::Close)
        }
        Message::Close(_) => Ok(MessageAction::Close),
        Message::Ping(bytes) => {
            ws.send(Message::Pong(bytes)).await?;
            Ok(MessageAction::Continue)
        }
        Message::Pong(_) => Ok(MessageAction::Continue),
        Message::Frame(_) => Ok(MessageAction::Continue),
    }
}

async fn authenticate_ws(state: &SharedState, jwt: Option<&str>) -> Result<Claims, AppError> {
    if !state.auth.is_auth_enabled() {
        return Ok(Claims::unauthenticated());
    }
    let jwt = jwt.ok_or(AppError::AuthRequired)?;
    state.auth.verify(jwt).await
}

fn authorize_scope(claims: &Claims, db_info: &DbInfo) -> Result<(), AppError> {
    if let Some(org) = claims.org.as_deref() {
        if org != db_info.organization {
            return Err(AppError::OrgScopeDenied);
        }
    }
    if let Some(group) = claims.grp.as_deref() {
        if group != db_info.group {
            return Err(AppError::OrgScopeDenied);
        }
    }
    Ok(())
}

async fn send_json<T: serde::Serialize>(ws: &mut Ws, value: &T) -> anyhow::Result<()> {
    ws.send(Message::Text(serde_json::to_string(value)?))
        .await?;
    Ok(())
}

fn status_response(status: StatusCode) -> HttpResponse {
    Response::builder()
        .status(status)
        .body(Full::from(Bytes::new()))
        .unwrap()
}

struct WsSession {
    db: Arc<dyn SqldAdapter>,
    db_info: DbInfo,
    claims: Claims,
    quota_exceeded: bool,
    streams: HashSet<u32>,
    stored_sql: HashMap<u32, String>,
}

impl WsSession {
    fn new(
        db: Arc<dyn SqldAdapter>,
        db_info: DbInfo,
        claims: Claims,
        quota_exceeded: bool,
    ) -> Self {
        Self {
            db,
            db_info,
            claims,
            quota_exceeded,
            streams: HashSet::new(),
            stored_sql: HashMap::new(),
        }
    }

    async fn handle_request(
        &mut self,
        stream_id: u32,
        body: RequestBody,
    ) -> Result<ResponseBody, AppError> {
        match body {
            RequestBody::OpenStream => {
                self.streams.insert(stream_id);
                Ok(ResponseBody::OpenStream)
            }
            RequestBody::CloseStream => {
                self.close_stream(stream_id).await;
                Ok(ResponseBody::CloseStream)
            }
            RequestBody::Execute { stmt } => {
                self.require_stream(stream_id)?;
                let stmt = self.resolve_stmt(stmt)?;
                let result = self.execute_stmt(&stmt).await?;
                Ok(ResponseBody::Execute { result })
            }
            RequestBody::Batch { batch } => {
                self.require_stream(stream_id)?;
                let mut step_results = Vec::with_capacity(batch.len());
                let mut step_errors = Vec::with_capacity(batch.len());
                for stmt in batch {
                    match self
                        .resolve_stmt(stmt)
                        .and_then(|stmt| self.precheck_stmt(&stmt).map(|_| stmt))
                    {
                        Ok(stmt) => match self.execute_stmt_unchecked(&stmt).await {
                            Ok(result) => {
                                step_results.push(Some(result));
                                step_errors.push(None);
                            }
                            Err(e) => {
                                step_results.push(None);
                                step_errors.push(Some(hrana_error(e)));
                            }
                        },
                        Err(e) => {
                            step_results.push(None);
                            step_errors.push(Some(hrana_error(e)));
                        }
                    }
                }
                Ok(ResponseBody::Batch {
                    step_results,
                    step_errors,
                })
            }
            RequestBody::Sequence { sql } => {
                self.require_stream(stream_id)?;
                if self.db_info.block_writes || self.quota_exceeded {
                    return Err(if self.quota_exceeded {
                        AppError::QuotaExceeded
                    } else {
                        AppError::PermissionDenied
                    });
                }
                if self.claims.resolve_access(&self.db_info.name) != AccessLevel::Rw {
                    return Err(AppError::PermissionDenied);
                }
                self.db.execute_batch(&sql).await?;
                Ok(ResponseBody::Sequence)
            }
            RequestBody::Describe { stmt } => {
                self.require_stream(stream_id)?;
                let stmt = self.resolve_stmt(stmt)?;
                let mut stmt = stmt;
                stmt.want_rows = true;
                match self.execute_stmt(&stmt).await {
                    Ok(result) => Ok(ResponseBody::Describe {
                        cols: result.cols,
                        params: vec![],
                    }),
                    Err(AppError::Sqld(_)) => Ok(ResponseBody::Describe {
                        cols: vec![],
                        params: vec![],
                    }),
                    Err(e) => Err(e),
                }
            }
            RequestBody::StoreSql { sql_id, sql } => {
                if self.stored_sql.contains_key(&sql_id) {
                    return Err(AppError::InvalidRequest);
                }
                self.stored_sql.insert(sql_id, sql);
                Ok(ResponseBody::StoreSql)
            }
            RequestBody::CloseSql { sql_id } => {
                self.stored_sql.remove(&sql_id);
                Ok(ResponseBody::CloseSql)
            }
            RequestBody::OpenCursor | RequestBody::FetchCursor | RequestBody::CloseCursor => {
                Err(AppError::ConfigError("NOT_IMPLEMENTED".to_string()))
            }
        }
    }

    async fn close_stream(&mut self, stream_id: u32) {
        self.streams.remove(&stream_id);
    }

    async fn rollback_open_streams(&mut self) {
        for _ in self.streams.drain() {
            let _ = self.db.execute_batch("ROLLBACK").await;
        }
    }

    fn require_stream(&self, stream_id: u32) -> Result<(), AppError> {
        if self.streams.contains(&stream_id) {
            Ok(())
        } else {
            Err(AppError::InvalidRequest)
        }
    }

    fn resolve_stmt(&self, stmt: WsStmt) -> Result<Stmt, AppError> {
        let sql = match (stmt.sql, stmt.sql_id) {
            (Some(sql), None) => sql,
            (None, Some(sql_id)) => self
                .stored_sql
                .get(&sql_id)
                .cloned()
                .ok_or(AppError::InvalidRequest)?,
            _ => return Err(AppError::InvalidRequest),
        };
        Ok(Stmt {
            sql,
            args: stmt.args,
            named_args: stmt.named_args,
            want_rows: stmt.want_rows,
        })
    }

    async fn execute_stmt(&self, stmt: &Stmt) -> Result<StmtResult, AppError> {
        self.precheck_stmt(stmt)?;
        self.execute_stmt_unchecked(stmt).await
    }

    async fn execute_stmt_unchecked(&self, stmt: &Stmt) -> Result<StmtResult, AppError> {
        if !stmt.named_args.is_empty() {
            return Err(AppError::InvalidRequest);
        }
        let args: Result<Vec<_>, _> = stmt.args.iter().map(hrana_to_sql).collect();
        let result = self.db.execute(&stmt.sql, args?, stmt.want_rows).await?;
        Ok(sql_to_stmt_result(result))
    }

    fn precheck_stmt(&self, stmt: &Stmt) -> Result<(), AppError> {
        let is_write = is_write_stmt(&stmt.sql);
        if is_write && self.db_info.block_writes {
            return Err(AppError::PermissionDenied);
        }
        if !is_write && self.db_info.block_reads {
            return Err(AppError::PermissionDenied);
        }
        if is_write && self.quota_exceeded {
            return Err(AppError::QuotaExceeded);
        }
        if is_write && self.claims.resolve_access(&self.db_info.name) != AccessLevel::Rw {
            return Err(AppError::PermissionDenied);
        }
        Ok(())
    }
}

fn is_write_stmt(sql: &str) -> bool {
    let upper = sql.trim_start().to_ascii_uppercase();
    matches!(
        upper.split_whitespace().next().unwrap_or(""),
        "INSERT"
            | "UPDATE"
            | "DELETE"
            | "CREATE"
            | "DROP"
            | "ALTER"
            | "REPLACE"
            | "PRAGMA"
            | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
            | "ATTACH"
    )
}

fn hrana_error(error: AppError) -> HranaError {
    match error {
        AppError::AuthRequired => HranaError {
            message: "authentication required".into(),
            code: "AUTH_REQUIRED".into(),
        },
        AppError::AuthInvalid => HranaError {
            message: "invalid or revoked token".into(),
            code: "AUTH_INVALID".into(),
        },
        AppError::AuthExpired => HranaError {
            message: "token expired".into(),
            code: "AUTH_EXPIRED".into(),
        },
        AppError::PermissionDenied => HranaError {
            message: "permission denied".into(),
            code: "PERMISSION_DENIED".into(),
        },
        AppError::QuotaExceeded => HranaError {
            message: "quota exceeded".into(),
            code: "QUOTA_EXCEEDED".into(),
        },
        AppError::DbNotFound(name) => HranaError {
            message: format!("database not found: {name}"),
            code: "DB_NOT_FOUND".into(),
        },
        AppError::InvalidRequest => HranaError {
            message: "invalid request".into(),
            code: "INVALID_REQUEST".into(),
        },
        AppError::ConfigError(message) if message == "NOT_IMPLEMENTED" => HranaError {
            message,
            code: "NOT_IMPLEMENTED".into(),
        },
        AppError::Sqld(message) => HranaError {
            code: sqld_error_code(&message),
            message,
        },
        other => HranaError {
            message: other.to_string(),
            code: "INTERNAL_ERROR".into(),
        },
    }
}

fn sqld_error_code(msg: &str) -> String {
    if msg.contains("UNIQUE constraint") {
        "SQLITE_CONSTRAINT".into()
    } else {
        "SQLITE_ERROR".into()
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Hello {
        jwt: Option<String>,
    },
    Request {
        request_id: i64,
        stream_id: u32,
        body: RequestBody,
    },
    CloseStream {
        stream_id: u32,
    },
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RequestBody {
    OpenStream,
    CloseStream,
    Execute { stmt: WsStmt },
    Batch { batch: Vec<WsStmt> },
    Sequence { sql: String },
    Describe { stmt: WsStmt },
    StoreSql { sql_id: u32, sql: String },
    CloseSql { sql_id: u32 },
    OpenCursor,
    FetchCursor,
    CloseCursor,
}

#[derive(Debug, serde::Deserialize)]
struct WsStmt {
    #[serde(default)]
    sql: Option<String>,
    #[serde(default)]
    sql_id: Option<u32>,
    #[serde(default)]
    args: Vec<crate::hrana::types::Value>,
    #[serde(default)]
    named_args: Vec<crate::hrana::types::NamedArg>,
    #[serde(default)]
    want_rows: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    HelloOk,
    HelloError {
        error: HranaError,
    },
    ResponseOk {
        request_id: i64,
        response: ResponseBody,
    },
    ResponseError {
        request_id: i64,
        error: HranaError,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResponseBody {
    OpenStream,
    CloseStream,
    Execute {
        result: StmtResult,
    },
    Batch {
        step_results: Vec<Option<StmtResult>>,
        step_errors: Vec<Option<HranaError>>,
    },
    Sequence,
    Describe {
        cols: Vec<crate::hrana::types::Col>,
        params: Vec<DescribeParam>,
    },
    StoreSql,
    CloseSql,
}

#[derive(Debug, serde::Serialize)]
struct DescribeParam {
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sqld_adapter::MockSqldAdapter;

    fn test_db_info() -> DbInfo {
        DbInfo {
            id: "db-id".into(),
            name: "default".into(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
            organization: "default".into(),
            group: "default".into(),
            location: "default".into(),
            delete_protection: false,
            block_reads: false,
            block_writes: false,
            allow_attach: true,
            legacy_name: false,
        }
    }

    fn test_claims() -> Claims {
        Claims::unauthenticated()
    }

    #[test]
    fn selects_first_supported_subprotocol() {
        let req = Request::builder()
            .header(
                header::SEC_WEBSOCKET_PROTOCOL,
                "hrana3-protobuf, hrana2, hrana1",
            )
            .body(())
            .unwrap();
        assert_eq!(select_protocol(&req), Some("hrana2"));
    }

    #[test]
    fn serializes_not_implemented_cursor_error_code() {
        let error = hrana_error(AppError::ConfigError("NOT_IMPLEMENTED".into()));
        assert_eq!(error.code, "NOT_IMPLEMENTED");
    }

    #[tokio::test]
    async fn rejects_execute_before_open_stream() {
        let mut session = WsSession::new(
            Arc::new(MockSqldAdapter),
            test_db_info(),
            test_claims(),
            false,
        );
        let result = session
            .handle_request(
                1,
                RequestBody::Execute {
                    stmt: WsStmt {
                        sql: Some("SELECT 1".into()),
                        sql_id: None,
                        args: vec![],
                        named_args: vec![],
                        want_rows: true,
                    },
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::InvalidRequest)));
    }

    #[tokio::test]
    async fn rejects_duplicate_store_sql_id() {
        let mut session = WsSession::new(
            Arc::new(MockSqldAdapter),
            test_db_info(),
            test_claims(),
            false,
        );
        assert!(session
            .handle_request(
                1,
                RequestBody::StoreSql {
                    sql_id: 7,
                    sql: "SELECT 1".into(),
                },
            )
            .await
            .is_ok());
        let result = session
            .handle_request(
                1,
                RequestBody::StoreSql {
                    sql_id: 7,
                    sql: "SELECT 2".into(),
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::InvalidRequest)));
    }
}
