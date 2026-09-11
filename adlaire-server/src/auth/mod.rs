pub mod middleware;

use std::collections::HashMap;

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

    pub fn can_write(&self) -> bool {
        matches!(self.a, AccessLevel::Rw)
    }
}

// ── AuthState ─────────────────────────────────────────────────────────────────

pub struct AuthState {
    pub(crate) secret_bytes: Option<Vec<u8>>,
}

impl AuthState {
    pub fn new(config: &Config) -> Self {
        Self { secret_bytes: config.jwt_secret_bytes.clone() }
    }

    pub fn is_auth_enabled(&self) -> bool {
        self.secret_bytes.is_some()
    }

    /// JWT 検証（Phase 4 で完全実装）
    pub async fn verify(&self, _raw_token: &str) -> Result<Claims, AppError> {
        Err(AppError::AuthInvalid)
    }
}

// ── tokens.json 読み込み ──────────────────────────────────────────────────────

/// 起動時に tokens.json を読み込んで AuthState に渡す
pub fn load_tokens(_config: &Config) -> anyhow::Result<Vec<()>> {
    // Phase 4 で実装
    Ok(vec![])
}
