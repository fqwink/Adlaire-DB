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
        let authorization = req
            .headers()
            .get(::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let auth_error = match authorization {
            Some(value) => match value.strip_prefix("Bearer ") {
                Some(token) if token == expected.as_str() => None,
                _ => Some(("AUTH_INVALID", "invalid or revoked token")),
            },
            None => Some(("AUTH_REQUIRED", "authentication required")),
        };
        if let Some((code, message)) = auth_error {
            let response = Ok(json_error(::http::StatusCode::UNAUTHORIZED, code, message));
            log_request(method.as_str(), &path, started, &response);
            return response;
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
        ("GET", "/admin/v1/organizations") => admin::organizations::list(req, state).await,
        ("POST", "/admin/v1/organizations") => admin::organizations::create(req, state).await,
        ("GET", p) if is_org_path(p) => admin::organizations::get(req, state).await,
        ("GET", "/admin/v1/groups") => admin::groups::list(req, state).await,
        ("POST", "/admin/v1/groups") => admin::groups::create(req, state).await,
        ("GET", "/admin/v1/locations") => admin::locations::list(req, state).await,
        ("POST", "/admin/v1/locations") => admin::locations::create(req, state).await,
        ("GET", "/admin/v1/quotas") => admin::quotas::list(req, state).await,
        ("PUT", p) if is_quota_path(p) => admin::quotas::put(req, state).await,
        ("GET", "/admin/v1/usage") => admin::usage::list(req, state).await,
        ("GET", "/v1/auth/validate") => admin::platform::auth_validate(req, state).await,
        ("GET", "/v1/locations") => admin::platform::locations(req, state).await,
        ("GET", "/v1/organizations") => admin::platform::organizations(req, state).await,
        ("PATCH", p) if is_turso_org_path(p) => admin::platform::patch_organization(req, state).await,
        ("GET", p) if is_turso_usage_path(p) => admin::platform::organization_usage(req, state).await,
        ("GET", p) if is_turso_groups_path(p) => admin::platform::groups(req, state).await,
        ("POST", p) if is_turso_groups_path(p) => admin::platform::create_group(req, state).await,
        ("GET", p) if is_turso_group_path(p) => admin::platform::group(req, state).await,
        ("PATCH", p) if is_turso_group_configuration_path(p) => admin::platform::patch_group_configuration(req, state).await,
        ("POST", p) if is_turso_group_rotate_path(p) => admin::platform::rotate_ok(req, state).await,
        ("POST", p) if is_turso_group_transfer_path(p) => admin::platform::unsupported(req, state).await,
        ("GET" | "PATCH" | "DELETE", p) if is_turso_group_transfer_path(p) => admin::platform::unsupported_method(req, state).await,
        ("GET", p) if is_turso_databases_path(p) => admin::platform::databases(req, state).await,
        ("POST", p) if is_turso_databases_path(p) => admin::platform::create_database(req, state).await,
        ("GET", p) if is_turso_database_path(p) => admin::platform::database(req, state).await,
        ("DELETE", p) if is_turso_database_path(p) => admin::platform::delete_database(req, state).await,
        ("PATCH", p) if is_turso_database_configuration_path(p) => admin::platform::patch_database_configuration(req, state).await,
        ("POST", p) if is_turso_database_token_path(p) => admin::platform::create_database_token(req, state).await,
        ("POST", p) if is_turso_database_rotate_path(p) => admin::platform::rotate_ok(req, state).await,
        ("GET", p) if is_turso_database_stats_path(p) => admin::platform::unsupported(req, state).await,
        ("POST" | "PATCH" | "DELETE", p) if is_turso_database_stats_path(p) => admin::platform::unsupported_method(req, state).await,
        ("GET", p) if is_turso_unsupported_read_path(p) => admin::platform::unsupported(req, state).await,
        ("POST" | "PATCH" | "DELETE", p) if is_turso_unsupported_write_path(p) => admin::platform::unsupported_method(req, state).await,
        ("POST", "/v1/upload") => admin::platform::unsupported(req, state).await,
        ("GET" | "PATCH" | "DELETE", "/v1/upload") => admin::platform::unsupported_method(req, state).await,
        ("GET", "/admin/v1/metrics") => admin::metrics::get(req, state).await,
        ("GET", p) if p.ends_with("/backup") => admin::backup::backup(req, state).await,
        ("POST", p) if p.ends_with("/restore") => admin::backup::restore(req, state).await,
        ("POST", p) if p.ends_with("/restore/point-in-time") => admin::backup::pitr(req, state).await,
        ("GET", p) if p.ends_with("/branches") => admin::branches::list(req, state).await,
        ("POST", p) if p.ends_with("/branches") => admin::branches::create(req, state).await,
        ("DELETE", p) if p.contains("/branches/") => admin::branches::delete(req, state).await,
        _ if path.starts_with("/v1/") => Ok(crate::error::AppError::EndpointNotFound.into_response()),
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

fn is_org_path(path: &str) -> bool {
    let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "organizations", _])
}

fn is_quota_path(path: &str) -> bool {
    let segs: Vec<_> = path.trim_start_matches('/').split('/').collect();
    matches!(segs.as_slice(), ["admin", "v1", "quotas", _])
}

fn is_turso_org_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _])
}

fn is_turso_usage_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "usage"])
}

fn is_turso_groups_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "groups"])
}

fn is_turso_group_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "groups", _])
}

fn is_turso_group_configuration_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "groups", _, "configuration"])
}

fn is_turso_group_rotate_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "groups", _, "auth", "rotate"])
}

fn is_turso_group_transfer_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "groups", _, "transfer"])
}

fn is_turso_databases_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases"])
}

fn is_turso_database_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases", _])
}

fn is_turso_database_configuration_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases", _, "configuration"])
}

fn is_turso_database_token_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases", _, "auth", "tokens"])
}

fn is_turso_database_rotate_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases", _, "auth", "rotate"])
}

fn is_turso_database_stats_path(path: &str) -> bool {
    matches!(segments(path).as_slice(), ["v1", "organizations", _, "databases", _, "stats"])
}

fn is_turso_unsupported_read_path(path: &str) -> bool {
    let segs = segments(path);
    matches!(
        segs.as_slice(),
        ["v1", "organizations", _, "members"]
            | ["v1", "organizations", _, "invites"]
            | ["v1", "organizations", _, "plans"]
            | ["v1", "organizations", _, "billing"]
            | ["v1", "organizations", _, "overages"]
            | ["v1", "organizations", _, "audit-logs"]
    )
}

fn is_turso_unsupported_write_path(path: &str) -> bool {
    let segs = segments(path);
    matches!(
        segs.as_slice(),
        ["v1", "organizations", _, "plans"]
            | ["v1", "organizations", _, "audit-logs"]
    )
}

fn segments(path: &str) -> Vec<&str> {
    path.trim_start_matches('/').split('/').collect()
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
