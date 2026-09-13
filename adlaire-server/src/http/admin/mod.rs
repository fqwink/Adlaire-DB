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

fn method_not_allowed() -> Result<HttpResponse, Infallible> {
    Ok(AppError::MethodNotAllowed.into_response())
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

fn empty_ok() -> HttpResponse {
    hyper::Response::builder()
        .status(::http::StatusCode::OK)
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

async fn parse_optional_json_body<T: serde::de::DeserializeOwned + Default>(
    req: Request<Incoming>,
) -> Result<T, AppError> {
    let bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|_| AppError::InvalidRequest)?
        .to_bytes();
    if bytes.is_empty() {
        return Ok(T::default());
    }
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

fn query_value(query: Option<&str>, key: &str) -> Option<String> {
    query.and_then(|query| {
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find_map(|(k, v)| (k == key && !v.is_empty()).then(|| v.to_string()))
    })
}

fn validate_token_name(value: &str) -> Result<(), AppError> {
    use regex::Regex;
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]{1,64}$").unwrap());
    if RE.is_match(value) {
        Ok(())
    } else {
        Err(AppError::InvalidRequest)
    }
}

fn parse_turso_expiry(
    value: Option<&str>,
    now: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<Option<chrono::DateTime<chrono::Utc>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value == "never" {
        return Ok(None);
    }
    let mut digits = String::new();
    let mut duration = chrono::Duration::zero();
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        let amount: i64 = digits.parse()?;
        anyhow::ensure!(amount > 0, "expiry amount must be positive");
        digits.clear();
        duration = duration
            + match ch {
                's' => chrono::Duration::seconds(amount),
                'm' => chrono::Duration::minutes(amount),
                'h' => chrono::Duration::hours(amount),
                'd' => chrono::Duration::days(amount),
                'w' => chrono::Duration::weeks(amount),
                _ => anyhow::bail!("unknown expiry unit"),
            };
    }
    anyhow::ensure!(digits.is_empty() && duration > chrono::Duration::zero(), "invalid expiry");
    Ok(Some(now + duration))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateDatabaseRequest {
    name:         String,
    organization: Option<String>,
    group:        Option<String>,
    location:     Option<String>,
    quota:        Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
struct DatabaseListResponse {
    databases: Vec<crate::db::DbInfo>,
}

#[derive(serde::Serialize)]
struct OrganizationsResponse {
    organizations: Vec<crate::db::meta::OrganizationInfo>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateOrganizationRequest {
    name: String,
    slug: Option<String>,
}

#[derive(serde::Serialize)]
struct GroupsResponse {
    groups: Vec<crate::db::meta::GroupInfo>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateGroupRequest {
    organization: String,
    name:         String,
    slug:         Option<String>,
    location:     Option<String>,
}

#[derive(serde::Serialize)]
struct LocationsResponse {
    locations: Vec<crate::db::meta::LocationInfo>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateLocationRequest {
    name:     String,
    provider: Option<String>,
    region:   Option<String>,
    primary:  Option<bool>,
}

#[derive(serde::Serialize)]
struct QuotasResponse {
    quotas: Vec<crate::db::meta::QuotaInfo>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PutQuotaRequest {
    storage_bytes:        Option<u64>,
    rows:                 Option<u64>,
    write_ops_per_minute: Option<u64>,
}

#[derive(serde::Serialize)]
struct UsageResponse {
    usage: Vec<crate::db::meta::UsageInfo>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateTokenRequest {
    access: AccessLevel,
    expiry: Option<String>,
    dbs: Option<std::collections::HashMap<String, AccessLevel>>,
    organization_scope: Option<String>,
    group_scope: Option<String>,
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
        let organization = body.organization.as_deref().unwrap_or("default");
        let group = body.group.as_deref().unwrap_or("default");
        let location = body.location.as_deref().unwrap_or("default");
        if let Some(quota) = &body.quota {
            if !quota.is_object() {
                return Ok(AppError::InvalidRequest.into_response());
            }
        }
        match state.db_mgr.create_scoped(&body.name, organization, group, location).await {
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
            org: body.organization_scope.clone(),
            grp: body.group_scope.clone(),
        };
        let token = match sign_claims(secret, &claims) {
            Ok(token) => token,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        let record = TokenRecord {
            id: token_id.clone(),
            access: body.access.clone(),
            dbs: body.dbs.clone(),
            organization_scope: body.organization_scope.clone(),
            group_scope: body.group_scope.clone(),
            source: Some("admin-api".to_string()),
            name: None,
            database: None,
            platform_token: false,
            token_hash: None,
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

pub mod organizations {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let organizations = state.db_mgr.organizations().await;
        Ok(crate::http::json_ok(&OrganizationsResponse { organizations }))
    }

    pub async fn create(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let body = match parse_json_body::<CreateOrganizationRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state.db_mgr.create_organization(body.name, body.slug).await {
            Ok(info) => Ok(json_created(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn get(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_param(&req, &["admin", "v1", "organizations"]) else {
            return Ok(crate::http::not_found());
        };
        match state
            .db_mgr
            .organizations()
            .await
            .into_iter()
            .find(|item| item.id == org || item.slug == org || item.name == org)
        {
            Some(info) => Ok(crate::http::json_ok(&info)),
            None => Ok(AppError::OrgNotFound(org).into_response()),
        }
    }
}

pub mod groups {
    use super::*;

    pub async fn list(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let org = query_value(req.uri().query(), "organization");
        match state.db_mgr.groups(org.as_deref()).await {
            Ok(groups) => Ok(crate::http::json_ok(&GroupsResponse { groups })),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn create(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let body = match parse_json_body::<CreateGroupRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state
            .db_mgr
            .create_group(body.organization, body.name, body.slug, body.location)
            .await
        {
            Ok(info) => Ok(json_created(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }
}

pub mod locations {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let locations = state.db_mgr.locations().await;
        Ok(crate::http::json_ok(&LocationsResponse { locations }))
    }

    pub async fn create(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let body = match parse_json_body::<CreateLocationRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state
            .db_mgr
            .create_location(body.name, body.provider, body.region, body.primary)
            .await
        {
            Ok(info) => Ok(json_created(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }
}

pub mod quotas {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let quotas = state.db_mgr.quotas().await;
        Ok(crate::http::json_ok(&QuotasResponse { quotas }))
    }

    pub async fn put(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(scope) = path_param(&req, &["admin", "v1", "quotas"]) else {
            return Ok(crate::http::not_found());
        };
        let Some((scope_type, scope)) = scope.split_once(':') else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<PutQuotaRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state
            .db_mgr
            .put_quota(
                scope_type.to_string(),
                scope.to_string(),
                body.storage_bytes,
                body.rows,
                body.write_ops_per_minute,
            )
            .await
        {
            Ok(info) => Ok(crate::http::json_ok(&info)),
            Err(e) => Ok(e.into_response()),
        }
    }
}

pub mod usage {
    use super::*;

    pub async fn list(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let usage = state.db_mgr.usage().await;
        Ok(crate::http::json_ok(&UsageResponse { usage }))
    }
}

pub mod platform {
    use super::*;

    const SPEC_VERSION: &str = "V.205";

    #[derive(serde::Serialize)]
    struct TursoOrganizationInfo {
        name:          String,
        slug:          String,
        #[serde(rename = "type")]
        kind:          String,
        overages:      bool,
        require_mfa:   bool,
        blocked_reads: bool,
    }

    #[derive(serde::Serialize)]
    struct TursoOrganizationUsage {
        uuid:          String,
        name:          String,
        slug:          String,
        storage_bytes: u64,
        databases:     usize,
    }

    #[derive(serde::Serialize)]
    struct TursoGroupInfo {
        name:              String,
        version:           String,
        uuid:              String,
        locations:         Vec<String>,
        primary:           String,
        delete_protection: bool,
    }

    #[derive(serde::Serialize)]
    struct TursoDatabaseInfo {
        #[serde(rename = "DbId")]
        db_id:         String,
        #[serde(rename = "Hostname")]
        hostname:      String,
        #[serde(rename = "Name")]
        name:          String,
        regions:       Vec<String>,
        #[serde(rename = "primaryRegion")]
        primary_region: String,
        block_reads:   bool,
        block_writes:  bool,
    }

    #[derive(Default, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CreatePlatformApiTokenRequest {
        organization: Option<String>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CreateTursoGroupRequest {
        name:     String,
        location: Option<String>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CreateTursoDatabaseRequest {
        name:              String,
        group:             Option<String>,
        seed:              Option<serde_json::Value>,
        remote_encryption: Option<serde_json::Value>,
        parent:            Option<serde_json::Value>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PatchOrganizationRequest {
        overages:    Option<bool>,
        require_mfa: Option<bool>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PatchDatabaseConfigurationRequest {
        size_limit:        Option<serde_json::Value>,
        delete_protection: Option<bool>,
        block_reads:       Option<bool>,
        block_writes:      Option<bool>,
        allow_attach:      Option<bool>,
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PatchGroupConfigurationRequest {
        delete_protection: bool,
    }

    pub async fn auth_validate(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        Ok(crate::http::json_ok(&serde_json::json!({ "exp": -1 })))
    }

    pub async fn create_api_token(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(token_name) = path_segment(&req, 3) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        if let Err(e) = validate_token_name(&token_name) {
            return Ok(e.into_response());
        }
        let body = match parse_optional_json_body::<CreatePlatformApiTokenRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        let organization_scope = match body.organization {
            Some(org) => {
                let Some(resolved) = state
                    .db_mgr
                    .organizations()
                    .await
                    .into_iter()
                    .find(|item| item.id == org || item.slug == org || item.name == org)
                    .map(|item| item.slug)
                else {
                    return Ok(AppError::OrgNotFound(org).into_response());
                };
                Some(resolved)
            }
            None => None,
        };

        let mut meta = match util::load_tokens(&state.config.data_dir) {
            Ok(meta) => meta,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        if meta
            .tokens
            .iter()
            .any(|token| token.platform_token && token.name.as_deref() == Some(token_name.as_str()))
        {
            return Ok(AppError::InvalidRequest.into_response());
        }

        let now = chrono::Utc::now();
        let secret = util::generate_platform_token_secret();
        let token_id = util::generate_token_id();
        meta.tokens.push(TokenRecord {
            id: token_id.clone(),
            access: AccessLevel::Rw,
            dbs: None,
            organization_scope,
            group_scope: None,
            source: Some("turso-platform-api-token".to_string()),
            name: Some(token_name.clone()),
            database: None,
            platform_token: true,
            token_hash: Some(util::platform_token_hash(&secret)),
            created_at: now,
            expires_at: None,
            revoked: false,
            revoked_at: None,
        });
        if let Err(e) = util::save_tokens(&state.config.data_dir, &meta) {
            return Ok(AppError::Internal(e).into_response());
        }

        Ok(crate::http::json_ok(&serde_json::json!({
            "name": token_name,
            "id": token_id,
            "token": secret,
        })))
    }

    pub async fn revoke_api_token(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(token_name) = path_segment(&req, 3) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        if let Err(e) = validate_token_name(&token_name) {
            return Ok(e.into_response());
        }
        let mut meta = match util::load_tokens(&state.config.data_dir) {
            Ok(meta) => meta,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        let Some(token) = meta
            .tokens
            .iter_mut()
            .find(|token| token.platform_token && token.name.as_deref() == Some(token_name.as_str()))
        else {
            return Ok(AppError::TokenNotFound(token_name).into_response());
        };
        if !token.revoked {
            token.revoked = true;
            token.revoked_at = Some(chrono::Utc::now());
        }
        if let Err(e) = util::save_tokens(&state.config.data_dir, &meta) {
            return Ok(AppError::Internal(e).into_response());
        }
        Ok(crate::http::json_ok(&serde_json::json!({ "token": token_name })))
    }

    pub async fn locations(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let locations = state
            .db_mgr
            .locations()
            .await
            .into_iter()
            .map(|loc| {
                let display = if loc.name == "default" {
                    "Self Hosted Default".to_string()
                } else {
                    format!("{} {}", loc.provider, title_case(&loc.region))
                };
                (loc.name, serde_json::Value::String(display))
            })
            .collect::<serde_json::Map<_, _>>();
        Ok(crate::http::json_ok(&serde_json::json!({ "locations": locations })))
    }

    pub async fn organizations(_req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let organizations: Vec<_> = state
            .db_mgr
            .organizations()
            .await
            .into_iter()
            .map(turso_org)
            .collect();
        Ok(crate::http::json_ok(&organizations))
    }

    pub async fn organization_usage(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let Some(info) = state
            .db_mgr
            .organizations()
            .await
            .into_iter()
            .find(|item| item.id == org || item.slug == org || item.name == org)
        else {
            return Ok(AppError::OrgNotFound(org).into_response());
        };
        let databases = match state.db_mgr.list_for_organization(&info.slug, None).await {
            Ok(databases) => databases,
            Err(e) => return Ok(e.into_response()),
        };
        let storage_bytes = databases.iter().map(|db| db.size_bytes).sum();
        Ok(crate::http::json_ok(&serde_json::json!({
            "organization": TursoOrganizationUsage {
                uuid: info.id,
                name: info.name,
                slug: info.slug,
                storage_bytes,
                databases: databases.len(),
            }
        })))
    }

    pub async fn patch_organization(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<PatchOrganizationRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        if body.overages.unwrap_or(false) || body.require_mfa.unwrap_or(false) {
            return Ok(AppError::InvalidRequest.into_response());
        }
        match state
            .db_mgr
            .organizations()
            .await
            .into_iter()
            .find(|item| item.id == org || item.slug == org || item.name == org)
        {
            Some(info) => Ok(crate::http::json_ok(&serde_json::json!({ "organization": turso_org(info) }))),
            None => Ok(AppError::OrgNotFound(org).into_response()),
        }
    }

    pub async fn groups(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        match state.db_mgr.groups(Some(&org)).await {
            Ok(groups) => {
                let groups: Vec<_> = groups.into_iter().map(turso_group).collect();
                Ok(crate::http::json_ok(&serde_json::json!({ "groups": groups })))
            }
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn create_group(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<CreateTursoGroupRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state.db_mgr.create_group(org, body.name, None, body.location).await {
            Ok(group) => Ok(crate::http::json_ok(&serde_json::json!({ "group": turso_group(group) }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn group(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let (Some(org), Some(group)) = (path_segment(&req, 2), path_segment(&req, 4)) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        match state.db_mgr.group_info(&org, &group).await {
            Ok(group) => Ok(crate::http::json_ok(&serde_json::json!({ "group": turso_group(group) }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn patch_group_configuration(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let (Some(org), Some(group)) = (path_segment(&req, 2), path_segment(&req, 4)) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<PatchGroupConfigurationRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        match state.db_mgr.update_group_configuration(&org, &group, body.delete_protection).await {
            Ok(group) => Ok(crate::http::json_ok(&serde_json::json!({ "delete_protection": group.delete_protection }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn databases(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let group = query_value(req.uri().query(), "group");
        match state.db_mgr.list_for_organization(&org, group.as_deref()).await {
            Ok(databases) => {
                let databases: Vec<_> = databases.into_iter().map(turso_database).collect();
                Ok(crate::http::json_ok(&serde_json::json!({ "databases": databases })))
            }
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn create_database(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(org) = path_segment(&req, 2) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<CreateTursoDatabaseRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        if body.seed.is_some() || body.remote_encryption.is_some() || body.parent.is_some() {
            return Ok(AppError::InvalidRequest.into_response());
        }
        let group = body.group.as_deref().unwrap_or("default");
        let location = match state.db_mgr.group_info(&org, group).await {
            Ok(group) => group.location,
            Err(e) => return Ok(e.into_response()),
        };
        match state.db_mgr.create_scoped(&body.name, &org, group, &location).await {
            Ok(database) => Ok(crate::http::json_ok(&serde_json::json!({ "database": turso_database(database) }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn database(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let (Some(org), Some(db)) = (path_segment(&req, 2), path_segment(&req, 4)) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        match state.db_mgr.get_info(&db).await {
            Ok(database) if database.organization == org => {
                Ok(crate::http::json_ok(&serde_json::json!({ "database": turso_database(database) })))
            }
            Ok(_) => Ok(AppError::DbNotFound(db).into_response()),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn delete_database(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let (Some(_org), Some(db)) = (path_segment(&req, 2), path_segment(&req, 4)) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        match state.db_mgr.delete(&db).await {
            Ok(()) => Ok(crate::http::json_ok(&serde_json::json!({ "database": db }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn patch_database_configuration(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(db) = path_segment(&req, 4) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let body = match parse_json_body::<PatchDatabaseConfigurationRequest>(req).await {
            Ok(body) => body,
            Err(e) => return Ok(e.into_response()),
        };
        if body.size_limit.is_none()
            && body.delete_protection.is_none()
            && body.block_reads.is_none()
            && body.block_writes.is_none()
            && body.allow_attach.is_none()
        {
            return Ok(AppError::InvalidRequest.into_response());
        }
        if let Some(size_limit) = &body.size_limit {
            if !size_limit.is_null() && !size_limit.is_string() {
                return Ok(AppError::InvalidRequest.into_response());
            }
        }
        match state
            .db_mgr
            .update_db_configuration(
                &db,
                body.delete_protection,
                body.block_reads,
                body.block_writes,
                body.allow_attach,
            )
            .await
        {
            Ok(info) => Ok(crate::http::json_ok(&serde_json::json!({
                "size_limit": body.size_limit,
                "allow_attach": info.allow_attach,
                "block_reads": info.block_reads,
                "block_writes": info.block_writes,
                "delete_protection": info.delete_protection,
            }))),
            Err(e) => Ok(e.into_response()),
        }
    }

    pub async fn create_database_token(req: Request<Incoming>, state: SharedState) -> Result<HttpResponse, Infallible> {
        let Some(db) = path_segment(&req, 4) else {
            return Ok(AppError::InvalidRequest.into_response());
        };
        let db_info = match state.db_mgr.get_info(&db).await {
            Ok(info) => info,
            Err(e) => return Ok(e.into_response()),
        };
        let secret = match state.config.jwt_secret_bytes.as_ref() {
            Some(secret) => secret,
            None => return Ok(AppError::AuthDisabled.into_response()),
        };
        let access = match query_value(req.uri().query(), "authorization").as_deref() {
            Some("read-only") => AccessLevel::Ro,
            Some("full-access") | None => AccessLevel::Rw,
            _ => return Ok(AppError::InvalidRequest.into_response()),
        };
        let now = chrono::Utc::now();
        let expires_at = match super::parse_turso_expiry(query_value(req.uri().query(), "expiration").as_deref(), now) {
            Ok(expires_at) => expires_at,
            Err(_) => return Ok(AppError::InvalidRequest.into_response()),
        };
        let token_id = util::generate_token_id();
        let mut dbs = std::collections::HashMap::new();
        dbs.insert(db.clone(), access.clone());
        let claims = Claims {
            iss: None,
            sub: token_id.clone(),
            iat: now.timestamp(),
            exp: expires_at.map(|t| t.timestamp()),
            a: access.clone(),
            dbs: Some(dbs.clone()),
            org: Some(db_info.organization.clone()),
            grp: Some(db_info.group.clone()),
        };
        let jwt = match sign_claims(secret, &claims) {
            Ok(token) => token,
            Err(e) => return Ok(AppError::Internal(e).into_response()),
        };
        let record = TokenRecord {
            id: token_id,
            access,
            dbs: Some(dbs),
            organization_scope: Some(db_info.organization),
            group_scope: Some(db_info.group),
            source: Some("turso-platform-api".to_string()),
            name: None,
            database: Some(db.clone()),
            platform_token: false,
            token_hash: None,
            created_at: now,
            expires_at,
            revoked: false,
            revoked_at: None,
        };
        if let Err(e) = util::append_token(&state.config.data_dir, record) {
            return Ok(AppError::Internal(e).into_response());
        }
        Ok(crate::http::json_ok(&serde_json::json!({ "jwt": jwt })))
    }

    pub async fn rotate_ok(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        Ok(super::empty_ok())
    }

    pub async fn unsupported(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        not_implemented()
    }

    pub async fn unsupported_method(_req: Request<Incoming>, _state: SharedState) -> Result<HttpResponse, Infallible> {
        method_not_allowed()
    }

    fn turso_org(org: crate::db::meta::OrganizationInfo) -> TursoOrganizationInfo {
        TursoOrganizationInfo {
            name: org.name,
            slug: org.slug,
            kind: "personal".to_string(),
            overages: false,
            require_mfa: false,
            blocked_reads: false,
        }
    }

    fn turso_group(group: crate::db::meta::GroupInfo) -> TursoGroupInfo {
        TursoGroupInfo {
            name: group.name,
            version: format!("adlaire-{SPEC_VERSION}"),
            uuid: group.id,
            locations: vec![group.location.clone()],
            primary: group.location,
            delete_protection: group.delete_protection,
        }
    }

    fn turso_database(db: crate::db::DbInfo) -> TursoDatabaseInfo {
        let hostname = format!("{}-{}.adlaire.local", db.name, db.organization);
        TursoDatabaseInfo {
            db_id: db.id,
            hostname,
            name: db.name,
            regions: vec![db.location.clone()],
            primary_region: db.location,
            block_reads: db.block_reads,
            block_writes: db.block_writes,
        }
    }

    fn path_segment(req: &Request<Incoming>, index: usize) -> Option<String> {
        req.uri()
            .path()
            .trim_start_matches('/')
            .split('/')
            .nth(index)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    fn title_case(value: &str) -> String {
        let mut chars = value.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
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
