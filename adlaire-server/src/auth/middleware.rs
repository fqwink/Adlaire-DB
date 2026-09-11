use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::{auth::Claims, error::AppError, state::SharedState};

/// JWT 認証 Extractor
/// - jwt_secret 未設定時: `Claims::unauthenticated()` を返す（認証なしモード）
/// - jwt_secret 設定時: Bearer トークンを検証（Phase 4 で完全実装）
pub struct Authenticated(pub Claims);

#[async_trait]
impl FromRequestParts<SharedState> for Authenticated {
    type Rejection = AppError;

    async fn from_request_parts(
        parts:  &mut Parts,
        state:  &SharedState,
    ) -> Result<Self, Self::Rejection> {
        if !state.auth.is_auth_enabled() {
            tracing::debug!("auth disabled — passing unauthenticated claims");
            return Ok(Authenticated(Claims::unauthenticated()));
        }

        let raw = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::AuthRequired)?;

        let claims = state.auth.verify(raw).await?;
        Ok(Authenticated(claims))
    }
}
