use axum::response::IntoResponse;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    AuthRequired,
    #[error("invalid or revoked token")]
    AuthInvalid,
    #[error("token expired")]
    AuthExpired,
    #[error("permission denied")]
    PermissionDenied,
    #[error("database not found: {0}")]
    DbNotFound(String),
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("database already exists: {0}")]
    DbAlreadyExists(String),
    #[error("invalid database name")]
    InvalidDbName,
    #[error("reserved database name")]
    DbReservedName,
    #[error("invalid request")]
    InvalidRequest,
    #[error("database is busy")]
    StorageBusy,
    #[error("replication timeout")]
    ReplicationTimeout,
    #[error("PITR not enabled")]
    PitrNotEnabled,
    #[error("frame not found")]
    FrameNotFound,
    #[error("restore integrity check failed")]
    RestoreIntegrityFailed,
    #[error("WAL frame corrupt")]
    RestoreFrameCorrupt,
    #[error("authentication is disabled")]
    AuthDisabled,
    #[error("config error: {0}")]
    ConfigError(String),
    // sqld::Error は Phase 3 以降で実型に置き換える
    #[error("sqld error: {0}")]
    Sqld(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, code) = match &self {
            Self::AuthRequired           => (StatusCode::UNAUTHORIZED,          "AUTH_REQUIRED"),
            Self::AuthInvalid            => (StatusCode::UNAUTHORIZED,          "AUTH_INVALID"),
            Self::AuthExpired            => (StatusCode::UNAUTHORIZED,          "AUTH_EXPIRED"),
            Self::PermissionDenied       => (StatusCode::FORBIDDEN,             "PERMISSION_DENIED"),
            Self::DbNotFound(_)          => (StatusCode::NOT_FOUND,             "DB_NOT_FOUND"),
            Self::TokenNotFound(_)       => (StatusCode::NOT_FOUND,             "TOKEN_NOT_FOUND"),
            Self::DbAlreadyExists(_)     => (StatusCode::CONFLICT,              "DB_ALREADY_EXISTS"),
            Self::InvalidDbName          => (StatusCode::BAD_REQUEST,           "INVALID_DB_NAME"),
            Self::DbReservedName         => (StatusCode::BAD_REQUEST,           "DB_RESERVED_NAME"),
            Self::InvalidRequest         => (StatusCode::BAD_REQUEST,           "INVALID_REQUEST"),
            Self::StorageBusy            => (StatusCode::SERVICE_UNAVAILABLE,   "STORAGE_BUSY"),
            Self::ReplicationTimeout     => (StatusCode::SERVICE_UNAVAILABLE,   "REPLICATION_TIMEOUT"),
            Self::PitrNotEnabled         => (StatusCode::SERVICE_UNAVAILABLE,   "PITR_NOT_ENABLED"),
            Self::FrameNotFound          => (StatusCode::NOT_FOUND,             "FRAME_NOT_FOUND"),
            Self::RestoreIntegrityFailed => (StatusCode::CONFLICT,              "RESTORE_INTEGRITY_FAILED"),
            Self::RestoreFrameCorrupt    => (StatusCode::CONFLICT,              "RESTORE_FRAME_CORRUPT"),
            Self::AuthDisabled           => (StatusCode::UNAUTHORIZED,          "AUTH_DISABLED"),
            Self::ConfigError(_)         => (StatusCode::INTERNAL_SERVER_ERROR, "CONFIG_ERROR"),
            Self::Sqld(_)                => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            Self::Internal(_)            => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        let body = axum::Json(serde_json::json!({
            "error": self.to_string(),
            "code":  code,
        }));
        (status, body).into_response()
    }
}
