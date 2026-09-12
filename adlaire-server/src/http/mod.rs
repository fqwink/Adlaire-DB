pub mod admin;
pub mod health;
pub mod pipeline;

use std::{convert::Infallible, time::Instant};

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};

use crate::state::SharedState;

pub type HttpResponse = Response<Full<Bytes>>;

pub async fn route(
    req: Request<Incoming>,
    state: SharedState,
) -> Result<HttpResponse, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = Instant::now();

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/v2/health") => health::handle(req, state).await,
        ("POST", "/v2/pipeline") => pipeline::handle(req, state, "default").await,
        ("POST", p) if p.ends_with("/v2/pipeline") => {
            match extract_db_name(p) {
                Some(db_name) => pipeline::handle(req, state, db_name).await,
                None => Ok(not_found()),
            }
        }
        _ => Ok(not_found()),
    };

    log_request(method.as_str(), &path, started, &response);
    response
}

pub async fn admin_route(
    req: Request<Incoming>,
    state: SharedState,
) -> Result<HttpResponse, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = Instant::now();

    if let Some(expected) = &state.config.admin_auth_token {
        let provided = req
            .headers()
            .get(::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));

        match provided {
            Some(token) if token == expected.as_str() => {}
            _ => {
                let response = Ok(json_error(
                    ::http::StatusCode::UNAUTHORIZED,
                    "AUTH_REQUIRED",
                    "unauthorized",
                ));
                log_request(method.as_str(), &path, started, &response);
                return response;
            }
        }
    }

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/admin/v1/databases") => admin::databases::list(req, state).await,
        ("POST", "/admin/v1/databases") => admin::databases::create(req, state).await,
        ("GET", p) if is_db_path(p) => admin::databases::get(req, state).await,
        ("DELETE", p) if is_db_path(p) => admin::databases::delete(req, state).await,
        ("GET", "/admin/v1/tokens") => admin::tokens::list(req, state).await,
        ("POST", "/admin/v1/tokens") => admin::tokens::create(req, state).await,
        ("GET", p) if is_token_path(p) => admin::tokens::get(req, state).await,
        ("DELETE", p) if is_token_path(p) => admin::tokens::revoke(req, state).await,
        ("GET", "/admin/v1/metrics") => admin::metrics::get(req, state).await,
        ("GET", p) if p.ends_with("/backup") => admin::backup::backup(req, state).await,
        ("POST", p) if p.ends_with("/restore") => admin::backup::restore(req, state).await,
        ("POST", p) if p.ends_with("/restore/point-in-time") => admin::backup::pitr(req, state).await,
        ("GET", p) if p.ends_with("/branches") => admin::branches::list(req, state).await,
        ("POST", p) if p.ends_with("/branches") => admin::branches::create(req, state).await,
        ("DELETE", p) if p.contains("/branches/") => admin::branches::delete(req, state).await,
        _ => Ok(not_found()),
    };

    log_request(method.as_str(), &path, started, &response);
    response
}

fn extract_db_name(path: &str) -> Option<&str> {
    let mut segs = path.trim_start_matches('/').split('/');
    match (segs.next(), segs.next(), segs.next(), segs.next()) {
        (Some(db), Some("v2"), Some("pipeline"), None) if !db.is_empty() => Some(db),
        _ => None,
    }
}

fn is_db_path(path: &str) -> bool {
    let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "databases", _])
}

fn is_token_path(path: &str) -> bool {
    let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "tokens", _])
}

pub fn json_ok<T: serde::Serialize>(body: &T) -> HttpResponse {
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    Response::builder()
        .status(::http::StatusCode::OK)
        .header(::http::header::CONTENT_TYPE, "application/json")
        .body(Full::from(Bytes::from(bytes)))
        .unwrap_or_else(|_| Response::new(Full::from(Bytes::new())))
}

pub fn json_error(status: ::http::StatusCode, code: &str, msg: &str) -> HttpResponse {
    let body = serde_json::to_vec(&serde_json::json!({
        "error": msg,
        "code": code,
    }))
    .unwrap_or_default();
    Response::builder()
        .status(status)
        .header(::http::header::CONTENT_TYPE, "application/json")
        .body(Full::from(Bytes::from(body)))
        .unwrap_or_else(|_| Response::new(Full::from(Bytes::new())))
}

pub fn not_found() -> HttpResponse {
    Response::builder()
        .status(::http::StatusCode::NOT_FOUND)
        .body(Full::from(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::from(Bytes::new())))
}

fn log_request(
    method: &str,
    path: &str,
    started: Instant,
    response: &Result<HttpResponse, Infallible>,
) {
    let status = response
        .as_ref()
        .map(|r| r.status().as_u16())
        .unwrap_or(::http::StatusCode::INTERNAL_SERVER_ERROR.as_u16());
    let duration_ms = started.elapsed().as_millis() as u64;
    tracing::info!(
        method,
        path,
        status,
        duration_ms,
        "http request"
    );
}
