// Phase 6〜7 で各ハンドラを実装する。現時点はすべて 501 を返す stub。

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::IntoResponse,
};

use crate::state::SharedState;

pub async fn admin_auth_middleware(
    State(state): State<SharedState>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    if let Some(expected) = &state.config.admin_auth_token {
        let provided = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        match provided {
            Some(token) if token == expected.as_str() => {}
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({
                        "error": "admin authentication required",
                        "code":  "AUTH_REQUIRED"
                    })),
                ).into_response();
            }
        }
    }
    next.run(req).await
}

pub mod databases {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn get()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn delete() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod tokens {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn get()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn revoke() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod metrics {
    use super::*;
    pub async fn get() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod backup {
    use super::*;
    pub async fn backup()  -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn restore() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn pitr()    -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}

pub mod branches {
    use super::*;
    pub async fn list()   -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn create() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
    pub async fn delete() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
}
