use std::{collections::HashMap, convert::Infallible, sync::Arc};

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
    db::{
        sqld_adapter::{from_libsql_value, to_libsql_params, SqlResult, SqldAdapter},
        validate_db_name, DbInfo,
    },
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
    streams: HashMap<u32, WsStream>,
    stored_sql: HashMap<u32, String>,
}

struct WsStream {
    conn: libsql::Connection,
    tx_open: bool,
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
            streams: HashMap::new(),
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
                self.open_stream(stream_id)?;
                Ok(ResponseBody::OpenStream)
            }
            RequestBody::CloseStream => {
                self.close_stream(stream_id).await;
                Ok(ResponseBody::CloseStream)
            }
            RequestBody::Execute { stmt } => {
                self.require_stream(stream_id)?;
                let stmt = self.resolve_stmt(stmt)?;
                self.precheck_stmt(&stmt)?;
                let stream = self.require_stream_mut(stream_id)?;
                let result = execute_stmt_on_stream(stream, &stmt).await?;
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
                        Ok(stmt) => match self.execute_stmt_for_stream(stream_id, &stmt).await {
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
                let stream = self.require_stream_mut(stream_id)?;
                stream.conn.execute_batch(&sql).await.map_err(libsql_err)?;
                apply_sequence_transaction_state(stream, &sql);
                Ok(ResponseBody::Sequence)
            }
            RequestBody::Describe { stmt } => {
                self.require_stream(stream_id)?;
                let stmt = self.resolve_stmt(stmt)?;
                self.precheck_stmt(&stmt)?;
                let stream = self.require_stream_mut(stream_id)?;
                match describe_stmt_on_stream(stream, &stmt).await {
                    Ok(cols) => Ok(ResponseBody::Describe {
                        cols,
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
                self.require_stream(stream_id)?;
                if self.stored_sql.contains_key(&sql_id) {
                    return Err(AppError::InvalidRequest);
                }
                self.stored_sql.insert(sql_id, sql);
                Ok(ResponseBody::StoreSql)
            }
            RequestBody::CloseSql { sql_id } => {
                self.require_stream(stream_id)?;
                self.stored_sql.remove(&sql_id);
                Ok(ResponseBody::CloseSql)
            }
            RequestBody::OpenCursor | RequestBody::FetchCursor | RequestBody::CloseCursor => {
                Err(AppError::ConfigError("NOT_IMPLEMENTED".to_string()))
            }
            RequestBody::Unknown => Err(AppError::InvalidRequest),
        }
    }

    fn open_stream(&mut self, stream_id: u32) -> Result<(), AppError> {
        if self.streams.contains_key(&stream_id) {
            return Err(AppError::InvalidRequest);
        }
        let conn = self.db.connect()?;
        self.streams.insert(
            stream_id,
            WsStream {
                conn,
                tx_open: false,
            },
        );
        Ok(())
    }

    async fn close_stream(&mut self, stream_id: u32) {
        if let Some(mut stream) = self.streams.remove(&stream_id) {
            rollback_stream_if_needed(&mut stream).await;
        }
    }

    async fn rollback_open_streams(&mut self) {
        for (_, mut stream) in self.streams.drain() {
            rollback_stream_if_needed(&mut stream).await;
        }
    }

    fn require_stream(&self, stream_id: u32) -> Result<(), AppError> {
        if self.streams.contains_key(&stream_id) {
            Ok(())
        } else {
            Err(AppError::InvalidRequest)
        }
    }

    fn require_stream_mut(&mut self, stream_id: u32) -> Result<&mut WsStream, AppError> {
        self.streams
            .get_mut(&stream_id)
            .ok_or(AppError::InvalidRequest)
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

    async fn execute_stmt_for_stream(
        &mut self,
        stream_id: u32,
        stmt: &Stmt,
    ) -> Result<StmtResult, AppError> {
        let stream = self.require_stream_mut(stream_id)?;
        execute_stmt_on_stream(stream, stmt).await
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

async fn execute_stmt_on_stream(
    stream: &mut WsStream,
    stmt: &Stmt,
) -> Result<StmtResult, AppError> {
    if !stmt.named_args.is_empty() {
        return Err(AppError::InvalidRequest);
    }
    let args: Result<Vec<_>, _> = stmt.args.iter().map(hrana_to_sql).collect();
    let result = execute_stmt_on_conn(&stream.conn, stmt, args?).await?;
    apply_stmt_transaction_state(stream, &stmt.sql);
    Ok(sql_to_stmt_result(result))
}

async fn describe_stmt_on_stream(
    stream: &mut WsStream,
    stmt: &Stmt,
) -> Result<Vec<crate::hrana::types::Col>, AppError> {
    if !stmt.named_args.is_empty() {
        return Err(AppError::InvalidRequest);
    }
    let prepared = stream.conn.prepare(&stmt.sql).await.map_err(libsql_err)?;
    Ok(prepared
        .columns()
        .iter()
        .map(|col| crate::hrana::types::Col {
            name: Some(col.name().to_string()),
            decltype: col.decl_type().map(str::to_string),
        })
        .collect())
}

async fn execute_stmt_on_conn(
    conn: &libsql::Connection,
    stmt: &Stmt,
    args: Vec<crate::db::sqld_adapter::SqlValue>,
) -> Result<SqlResult, AppError> {
    let params = to_libsql_params(args);
    if stmt.want_rows {
        let mut rows = conn.query(&stmt.sql, params).await.map_err(libsql_err)?;

        let col_count = rows.column_count();
        let cols: Vec<(Option<String>, Option<String>)> = (0..col_count)
            .map(|i| {
                let name = rows.column_name(i).map(|s| s.to_string());
                (name, None)
            })
            .collect();

        let mut result_rows = vec![];
        while let Some(row) = rows.next().await.map_err(libsql_err)? {
            let cells = (0..col_count)
                .map(|i| {
                    let v = row.get_value(i).unwrap_or(libsql::Value::Null);
                    from_libsql_value(v)
                })
                .collect();
            result_rows.push(cells);
        }

        Ok(SqlResult {
            cols,
            rows: result_rows,
            rows_affected: 0,
            last_insert_rowid: None,
        })
    } else {
        let rows_affected = conn.execute(&stmt.sql, params).await.map_err(libsql_err)?;
        Ok(SqlResult {
            cols: vec![],
            rows: vec![],
            rows_affected,
            last_insert_rowid: Some(conn.last_insert_rowid()),
        })
    }
}

fn libsql_err(e: libsql::Error) -> AppError {
    let msg = e.to_string();
    if msg.contains("locked") || msg.contains("busy") {
        AppError::StorageBusy
    } else {
        AppError::Sqld(msg)
    }
}

async fn rollback_stream_if_needed(stream: &mut WsStream) {
    if stream.tx_open {
        if stream.conn.execute_batch("ROLLBACK").await.is_ok() {
            stream.tx_open = false;
        }
    }
}

fn apply_stmt_transaction_state(stream: &mut WsStream, sql: &str) {
    match transaction_action(sql) {
        TransactionAction::Begin => stream.tx_open = true,
        TransactionAction::End => stream.tx_open = false,
        TransactionAction::None => {}
    }
}

fn apply_sequence_transaction_state(stream: &mut WsStream, sql: &str) {
    apply_stmt_transaction_state(stream, sql);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionAction {
    None,
    Begin,
    End,
}

fn transaction_action(sql: &str) -> TransactionAction {
    match first_sql_keyword(sql).as_deref() {
        Some("BEGIN") => TransactionAction::Begin,
        Some("COMMIT") | Some("END") | Some("ROLLBACK") => TransactionAction::End,
        _ => TransactionAction::None,
    }
}

fn is_write_stmt(sql: &str) -> bool {
    let Some(first) = first_sql_keyword(sql) else {
        return false;
    };
    if first == "BEGIN" {
        let second = sql
            .trim_start()
            .split_whitespace()
            .nth(1)
            .map(|s| s.trim_matches(';').to_ascii_uppercase());
        return matches!(second.as_deref(), Some("IMMEDIATE" | "EXCLUSIVE"));
    }
    matches!(
        first.as_str(),
        "INSERT"
            | "UPDATE"
            | "DELETE"
            | "CREATE"
            | "DROP"
            | "ALTER"
            | "REPLACE"
            | "PRAGMA"
            | "COMMIT"
            | "ROLLBACK"
            | "ATTACH"
    )
}

fn first_sql_keyword(sql: &str) -> Option<String> {
    sql.trim_start()
        .split_whitespace()
        .next()
        .map(|s| s.trim_matches(';').to_ascii_uppercase())
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
    Execute {
        stmt: WsStmt,
    },
    Batch {
        batch: Vec<WsStmt>,
    },
    Sequence {
        sql: String,
    },
    Describe {
        stmt: WsStmt,
    },
    StoreSql {
        sql_id: u32,
        sql: String,
    },
    CloseSql {
        sql_id: u32,
    },
    OpenCursor,
    FetchCursor,
    CloseCursor,
    #[serde(other)]
    Unknown,
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
    use crate::{
        db::sqld_adapter::{MockSqldAdapter, RealSqldAdapter},
        hrana::types::Value,
    };
    use std::path::PathBuf;

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

    fn temp_db_path(test_name: &str) -> PathBuf {
        let path = std::env::temp_dir()
            .join("adlaire-db-phase9-ws")
            .join(format!("{}-{}.db", test_name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }

    async fn real_session(test_name: &str) -> (WsSession, PathBuf) {
        let path = temp_db_path(test_name);
        let adapter = RealSqldAdapter::open(&path, 5000, false).await.unwrap();
        (
            WsSession::new(Arc::new(adapter), test_db_info(), test_claims(), false),
            path,
        )
    }

    async fn open_stream(session: &mut WsSession, stream_id: u32) {
        assert!(session
            .handle_request(stream_id, RequestBody::OpenStream)
            .await
            .is_ok());
    }

    async fn execute_sql(
        session: &mut WsSession,
        stream_id: u32,
        sql: &str,
        want_rows: bool,
    ) -> ResponseBody {
        session
            .handle_request(
                stream_id,
                RequestBody::Execute {
                    stmt: WsStmt {
                        sql: Some(sql.into()),
                        sql_id: None,
                        args: vec![],
                        named_args: vec![],
                        want_rows,
                    },
                },
            )
            .await
            .unwrap()
    }

    fn first_integer(response: ResponseBody) -> String {
        let ResponseBody::Execute { result } = response else {
            panic!("expected execute response");
        };
        let Value::Integer { value } = &result.rows[0][0] else {
            panic!("expected integer cell");
        };
        value.clone()
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
        let (mut session, path) = real_session("duplicate-store-sql").await;
        open_stream(&mut session, 1).await;

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

        drop(session);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn rejects_store_sql_before_open_stream() {
        let mut session = WsSession::new(
            Arc::new(MockSqldAdapter),
            test_db_info(),
            test_claims(),
            false,
        );
        let result = session
            .handle_request(
                1,
                RequestBody::StoreSql {
                    sql_id: 7,
                    sql: "SELECT 1".into(),
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::InvalidRequest)));
    }

    #[tokio::test]
    async fn rejects_unknown_request_body_without_closing_session() {
        let (mut session, path) = real_session("unknown-request").await;
        open_stream(&mut session, 1).await;

        let result = session.handle_request(1, RequestBody::Unknown).await;
        assert!(matches!(result, Err(AppError::InvalidRequest)));

        let select = execute_sql(&mut session, 1, "SELECT 1", true).await;
        assert_eq!(first_integer(select), "1");

        drop(session);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn store_sql_is_connection_local_and_close_invalidates_id() {
        let (mut session, path) = real_session("store-sql-close").await;
        open_stream(&mut session, 1).await;

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

        let stored = session
            .handle_request(
                1,
                RequestBody::Execute {
                    stmt: WsStmt {
                        sql: None,
                        sql_id: Some(7),
                        args: vec![],
                        named_args: vec![],
                        want_rows: true,
                    },
                },
            )
            .await
            .unwrap();
        assert_eq!(first_integer(stored), "1");

        assert!(session
            .handle_request(1, RequestBody::CloseSql { sql_id: 7 })
            .await
            .is_ok());
        let result = session
            .handle_request(
                1,
                RequestBody::Execute {
                    stmt: WsStmt {
                        sql: None,
                        sql_id: Some(7),
                        args: vec![],
                        named_args: vec![],
                        want_rows: true,
                    },
                },
            )
            .await;
        assert!(matches!(result, Err(AppError::InvalidRequest)));

        drop(session);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn websocket_stream_commit_persists_rows() {
        let (mut session, path) = real_session("commit-persists").await;
        open_stream(&mut session, 1).await;

        execute_sql(
            &mut session,
            1,
            "CREATE TABLE items (id INTEGER PRIMARY KEY)",
            false,
        )
        .await;
        execute_sql(&mut session, 1, "BEGIN", false).await;
        execute_sql(&mut session, 1, "INSERT INTO items DEFAULT VALUES", false).await;
        execute_sql(&mut session, 1, "COMMIT", false).await;

        open_stream(&mut session, 2).await;
        let count = execute_sql(&mut session, 2, "SELECT COUNT(*) FROM items", true).await;
        assert_eq!(first_integer(count), "1");

        drop(session);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn websocket_stream_close_rolls_back_open_transaction() {
        let (mut session, path) = real_session("close-rolls-back").await;
        open_stream(&mut session, 1).await;

        execute_sql(
            &mut session,
            1,
            "CREATE TABLE items (id INTEGER PRIMARY KEY)",
            false,
        )
        .await;
        execute_sql(&mut session, 1, "BEGIN", false).await;
        execute_sql(&mut session, 1, "INSERT INTO items DEFAULT VALUES", false).await;
        session.close_stream(1).await;

        open_stream(&mut session, 2).await;
        let count = execute_sql(&mut session, 2, "SELECT COUNT(*) FROM items", true).await;
        assert_eq!(first_integer(count), "0");

        drop(session);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn websocket_streams_use_independent_connections() {
        let (mut session, path) = real_session("independent-connections").await;
        open_stream(&mut session, 1).await;
        open_stream(&mut session, 2).await;

        execute_sql(
            &mut session,
            1,
            "CREATE TABLE items (id INTEGER PRIMARY KEY)",
            false,
        )
        .await;
        execute_sql(&mut session, 1, "BEGIN", false).await;
        execute_sql(&mut session, 1, "INSERT INTO items DEFAULT VALUES", false).await;

        let uncommitted = execute_sql(&mut session, 2, "SELECT COUNT(*) FROM items", true).await;
        assert_eq!(first_integer(uncommitted), "0");

        execute_sql(&mut session, 1, "COMMIT", false).await;
        let committed = execute_sql(&mut session, 2, "SELECT COUNT(*) FROM items", true).await;
        assert_eq!(first_integer(committed), "1");

        drop(session);
        let _ = std::fs::remove_file(path);
    }
}
