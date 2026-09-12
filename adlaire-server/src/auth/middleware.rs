use hyper::{body::Incoming, Request};

use crate::{auth::Claims, error::AppError, state::SharedState};

pub async fn extract_claims(
    req: &Request<Incoming>,
    state: &SharedState,
) -> Result<Claims, AppError> {
    if !state.auth.is_auth_enabled() {
        tracing::debug!("auth disabled — passing unauthenticated claims");
        return Ok(Claims::unauthenticated());
    }

    let raw = req
        .headers()
        .get(::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(AppError::AuthRequired)?;

    state.auth.verify(raw).await
}
