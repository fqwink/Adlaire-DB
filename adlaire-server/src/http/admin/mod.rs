use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, Request};

use crate::{
    auth::{sign_claims, AccessLevel, Claims},
    db::validate_db_name,
    error::AppError,
    http::HttpResponse,
    state::SharedState,
    token::util::{self, TokenRecord},
};

fn not_implemented() -> Result<HttpResponse, Infallible> {
    Ok(crate::http::json_error(
        ::http::StatusCode::NOT_IMPLEMENTED,
        "NOT_IMPLEMENTED",
        "not implemented",
    ))
}

fn json_created<T: serde::Serialize>(body: &T) -> HttpResponse {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    hyper::Response::builder()
        .status(::http::StatusCode::CREATED)
        .header(::http::header::CONTENT_TYPE, "application/json")
        .body(Full::from(Bytes::from(bytes)))
        .unwrap_or_else(|_| hyper::Response::new(Full::from(Bytes::new())))
}

fn no_content() -> HttpResponse {
    hyper::Response::builder()
        .status(::http::StatusCode::NO_CONTENT)
        .body(Full::from(Bytes::new()))
        .unwrap_or_else(|_| hyper::Response::new(Full::from(Bytes::new())))
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

fn path_param(req: &Request<Incoming>, prefix: &[&str]) -> Option<String> {
    let segs: Vec<_> = req.uri().path().trim_start_matches('/').split('/').collect();
    if segs.len() != prefix.len() + 1 || &segs[..prefix.len()] != prefix {
        return None;
    }
    Some(segs[prefix.len()].to_string())
}

fn reject_internal_default_db(name: &str) -> Result<(), AppError> {
    if name == "default" {
        return Err(AppError::DbNotFound(name.to_string()));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct CreateDatabaseRequest {
    name: String,
}

#[derive(serde::Serialize)]
struct DatabaseListResponse {
    databases: Vec<crate::db::DbInfo>,
}

#[derive(serde::Deserialize)]
struct CreateTokenRequest {
    access: AccessLevel,
    expiry: Option<String>,
    dbs: Option<std::collections::HashMap<String, AccessLevel>>,
}

#[derive(serde::Serialize)]
struct CreateTokenResponse {
    id: String,
    token: String,
    access: AccessLevel,
    dbs: Option<std::collections::HashMap<String, AccessLevel>>,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    revoked: bool,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(serde::Serialize)]
struct TokenListResponse {
    tokens: Vec<TokenRecord>,
}

pub mod databases {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let databases = state
            .db_mgr
            .list()
            .await
            .into_iter()
            .filter(|db| db.name != "default")
            .collect();
        Ok(crate::http::json_ok(&DatabaseListResponse { databases }))
    }

    pub async fn create(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let body = match parse_json_body::<CreateDatabaseRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state.db_mgr.create(&body.name).await {
            Ok(info) => Ok(json_created(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn get(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(name) = path_param(&req, &["admin", "v1", "databases"]) else {
            return Ok(crate::http::not_found());
        };
        if let Err(e) = validate_db_name(&name) {
            return Ok(e.into_response());
        }
        if let Err(e) = reject_internal_default_db(&name) {
            return Ok(e.into_response());
        }
        match state.db_mgr.get_info(&name).await {
            Ok(info) => Ok(crate::http::json_ok(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn delete(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(name) = path_param(&req, &["admin", "v1", "databases"]) else {
            return Ok(crate::http::not_found());
        };
        if let Err(e) = validate_db_name(&name) {
            return Ok(e.into_response());
        }
        if let Err(e) = reject_internal_default_db(&name) {
            return Ok(e.into_response());
        }
        match state.db_mgr.delete(&name).await {
            Ok(()) => Ok(no_content()),
            Err(e) => Ok(e.into_response()),
        }
    }
}

pub mod tokens {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        match util::load_tokens(&state.config.data_dir) {
            Ok(meta) => Ok(crate::http::json_ok(&TokenListResponse { tokens: meta.tokens })),
            Err(e) => Ok(AppError::Internal(e).into_response()),
        }
    }

    pub async fn create(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let secret = match state.config.jwt_secret_bytes.as_ref() {
            Some(secret) => secret,
            None => return Ok(AppError::AuthDisabled.into_response()),
        };
        let body = match parse_json_body::<CreateTokenRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        if let Some(dbs) = &body.dbs {
            for db_name in dbs.keys() {
                if let Err(e) = validate_db_name(db_name) {
                    return Ok(e.into_response());
                }
            }
        }

        let now = chrono::Utc::now();
        let expires_at = match util::parse_expiry(body.expiry.as_deref(), now) {
            Ok(expires_at) => expires_at,
            Err(_) => return Ok(AppError::InvalidRequest.into_response()),
        };
        let token_id = util::generate_token_id();
        let claims = Claims {
            iss: None,
            sub: token_id.clone(),
            iat: now.timestamp(),
            exp: expires_at.map(|t| t.timestamp()),
            a: body.access.clone(),
            dbs: body.dbs.clone(),
        };
        let token = match sign_claims(secret, &claims) {
            Ok(token) => token,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        let record = TokenRecord {
            id: token_id.clone(),
            access: body.access.clone(),
            dbs: body.dbs.clone(),
            created_at: now,
            expires_at,
            revoked: false,
            revoked_at: None,
        };
        if let Err(e) = util::append_token(&state.config.data_dir, record.clone()) {
            return Ok(AppError::Internal(e).into_response());
        }

        Ok(json_created(&CreateTokenResponse {
            id: token_id,
            token,
            access: record.access,
            dbs: record.dbs,
            created_at: record.created_at,
            expires_at: record.expires_at,
            revoked: record.revoked,
            revoked_at: record.revoked_at,
        }))
    }

    pub async fn get(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(id) = path_param(&req, &["admin", "v1", "tokens"]) else {
            return Ok(crate::http::not_found());
        };
        match util::load_tokens(&state.config.data_dir) {
            Ok(meta) => match meta.tokens.into_iter().find(|token| token.id == id) {
                Some(token) => Ok(crate::http::json_ok(&token)),
                None => Ok(AppError::TokenNotFound(id).into_response()),
            },
            Err(e) => Ok(AppError::Internal(e).into_response()),
        }
    }

    pub async fn revoke(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(id) = path_param(&req, &["admin", "v1", "tokens"]) else {
            return Ok(crate::http::not_found());
        };
        let mut meta = match util::load_tokens(&state.config.data_dir) {
            Ok(meta) => meta,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        let Some(token) = meta.tokens.iter_mut().find(|token| token.id == id) else {
            return Ok(AppError::TokenNotFound(id).into_response());
        };
        if !token.revoked {
            token.revoked = true;
            token.revoked_at = Some(chrono::Utc::now());
            if let Err(e) = util::save_tokens(&state.config.data_dir, &meta) {
                return Ok(AppError::Internal(e).into_response());
            }
        }
        state.auth.revoke(id).await;
        Ok(no_content())
    }
}

pub mod metrics {
    use super::*;

    pub async fn get(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod backup {
    use super::*;

    pub async fn backup(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn restore(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn pitr(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}

pub mod branches {
    use super::*;

    pub async fn list(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn create(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn delete(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }
}
