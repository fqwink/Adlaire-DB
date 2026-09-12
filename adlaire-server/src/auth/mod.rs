pub mod middleware;

use std::collections::{HashMap, HashSet};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::hmac;

use crate::{config::Config, error::AppError};

// ── アクセスレベル ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel {
    Rw,
    Ro,
}

// ── JWT クレーム ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub iss: Option<String>,
    pub sub: String,
    pub iat: i64,
    pub exp: Option<i64>,
    pub a:   AccessLevel,
    pub dbs: Option<HashMap<String, AccessLevel>>,
}

impl Claims {
    /// 認証無効モード用（jwt_secret 未設定時）
    pub fn unauthenticated() -> Self {
        Self {
            iss: None,
            sub: String::new(),
            iat: 0,
            exp: None,
            a:   AccessLevel::Rw,
            dbs: None,
        }
    }

    /// DB 名に対するアクセスレベルを解決する
    pub fn resolve_access(&self, db_name: &str) -> AccessLevel {
        match &self.dbs {
            Some(dbs) => dbs.get(db_name).cloned().unwrap_or(self.a.clone()),
            None      => self.a.clone(),
        }
    }

    /// DB 名を考慮した書き込み権限チェック（per-DB レベル優先）
    pub fn can_write_db(&self, db_name: &str) -> bool {
        self.resolve_access(db_name) == AccessLevel::Rw
    }
}

// ── AuthState ─────────────────────────────────────────────────────────────────

pub struct AuthState {
    pub(crate) secret_bytes: Option<Vec<u8>>,
    revoked: HashSet<String>,
}

impl AuthState {
    pub fn new(config: &Config, revoked: HashSet<String>) -> Self {
        Self {
            secret_bytes: config.jwt_secret_bytes.clone(),
            revoked,
        }
    }

    pub fn is_auth_enabled(&self) -> bool {
        self.secret_bytes.is_some()
    }

    pub async fn verify(&self, raw_token: &str) -> Result<Claims, AppError> {
        let secret = self.secret_bytes.as_ref().ok_or(AppError::AuthInvalid)?;
        let mut parts = raw_token.split('.');
        let header_b64 = parts.next().ok_or(AppError::AuthInvalid)?;
        let claims_b64 = parts.next().ok_or(AppError::AuthInvalid)?;
        let sig_b64 = parts.next().ok_or(AppError::AuthInvalid)?;
        if parts.next().is_some() {
            return Err(AppError::AuthInvalid);
        }

        let signed = format!("{header_b64}.{claims_b64}");
        let sig = URL_SAFE_NO_PAD
            .decode(sig_b64)
            .map_err(|_| AppError::AuthInvalid)?;
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        hmac::verify(&key, signed.as_bytes(), &sig).map_err(|_| AppError::AuthInvalid)?;

        let header: serde_json::Value = decode_json(header_b64)?;
        if header.get("alg").and_then(|v| v.as_str()) != Some("HS256") {
            return Err(AppError::AuthInvalid);
        }

        let claims: Claims = decode_json(claims_b64)?;
        if let Some(exp) = claims.exp {
            if exp <= chrono::Utc::now().timestamp() {
                return Err(AppError::AuthExpired);
            }
        }
        if self.revoked.contains(&claims.sub) {
            return Err(AppError::AuthInvalid);
        }
        Ok(claims)
    }
}

// ── tokens.json 読み込み ──────────────────────────────────────────────────────

/// 起動時に tokens.json を読み込んで AuthState に渡す
pub fn load_revoked_tokens(config: &Config) -> anyhow::Result<HashSet<String>> {
    let meta = crate::token::util::load_tokens(&config.data_dir)?;
    Ok(meta
        .tokens
        .into_iter()
        .filter(|t| t.revoked)
        .map(|t| t.id)
        .collect())
}

pub fn sign_claims(secret: &[u8], claims: &Claims) -> anyhow::Result<String> {
    let header = serde_json::json!({ "alg": "HS256", "typ": "JWT" });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims)?);
    let signed = format!("{header_b64}.{claims_b64}");
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let sig = hmac::sign(&key, signed.as_bytes());
    Ok(format!(
        "{signed}.{}",
        URL_SAFE_NO_PAD.encode(sig.as_ref())
    ))
}

fn decode_json<T: serde::de::DeserializeOwned>(b64: &str) -> Result<T, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|_| AppError::AuthInvalid)?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::AuthInvalid)
}
