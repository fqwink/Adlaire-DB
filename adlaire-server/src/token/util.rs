use std::{collections::HashMap, fs::OpenOptions, io::Write, path::Path};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use ring::digest;
use uuid::Uuid;

use crate::auth::AccessLevel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenRecord {
    pub id:         String,
    pub access:     AccessLevel,
    pub dbs:        Option<HashMap<String, AccessLevel>>,
    #[serde(default)]
    pub organization_scope: Option<String>,
    #[serde(default)]
    pub group_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(default)]
    pub platform_token: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked:    bool,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokensMeta {
    pub tokens: Vec<TokenRecord>,
}

pub fn generate_token_id() -> String {
    format!("tok_{}", Uuid::new_v4().simple())
}

pub fn generate_platform_token_secret() -> String {
    format!("adlpt_{}", Uuid::new_v4().simple())
}

pub fn platform_token_hash(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(digest::digest(&digest::SHA256, token.as_bytes()).as_ref())
}

pub fn parse_expiry(s: Option<&str>, now: DateTime<Utc>) -> anyhow::Result<Option<DateTime<Utc>>> {
    let Some(s) = s else {
        return Ok(None);
    };
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let (num, unit) = s.split_at(s.len().saturating_sub(1));
    let value: i64 = num.parse()?;
    anyhow::ensure!(value > 0, "expiry must be positive");
    let duration = match unit {
        "s" => Duration::seconds(value),
        "m" => Duration::minutes(value),
        "h" => Duration::hours(value),
        "d" => Duration::days(value),
        _ => anyhow::bail!("unknown expiry unit: {unit}. Use s, m, h, or d"),
    };
    Ok(Some(now + duration))
}

pub fn load_tokens(data_dir: &Path) -> anyhow::Result<TokensMeta> {
    let path = data_dir.join("meta").join("tokens.json");
    if !path.exists() {
        return Ok(TokensMeta::default());
    }
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn save_tokens(data_dir: &Path, meta: &TokensMeta) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;
    let path = meta_dir.join("tokens.json");
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(meta)?;
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, &path)?;
    let dir = OpenOptions::new().read(true).open(meta_dir)?;
    dir.sync_all()?;
    Ok(())
}

pub fn append_token(data_dir: &Path, record: TokenRecord) -> anyhow::Result<()> {
    let mut meta = load_tokens(data_dir)?;
    meta.tokens.push(record);
    save_tokens(data_dir, &meta)
}

pub fn revoke_database_tokens(data_dir: &Path, database: &str) -> anyhow::Result<usize> {
    let mut meta = load_tokens(data_dir)?;
    let now = Utc::now();
    let mut revoked = 0;
    for token in &mut meta.tokens {
        if token.platform_token || token.revoked {
            continue;
        }
        let database_matches = token.database.as_deref() == Some(database)
            || token
                .dbs
                .as_ref()
                .map(|dbs| dbs.contains_key(database))
                .unwrap_or(false);
        if database_matches {
            token.revoked = true;
            token.revoked_at = Some(now);
            revoked += 1;
        }
    }
    if revoked > 0 {
        save_tokens(data_dir, &meta)?;
    }
    Ok(revoked)
}

pub fn revoke_group_tokens(
    data_dir: &Path,
    organization: &str,
    group: &str,
    databases: &[String],
) -> anyhow::Result<usize> {
    let mut meta = load_tokens(data_dir)?;
    let now = Utc::now();
    let mut revoked = 0;
    for token in &mut meta.tokens {
        if token.platform_token || token.revoked {
            continue;
        }
        let scoped_to_group = token.organization_scope.as_deref() == Some(organization)
            && token.group_scope.as_deref() == Some(group);
        let scoped_to_group_database = token
            .database
            .as_ref()
            .map(|database| databases.iter().any(|item| item == database))
            .unwrap_or(false)
            || token
                .dbs
                .as_ref()
                .map(|dbs| databases.iter().any(|database| dbs.contains_key(database)))
                .unwrap_or(false);
        if scoped_to_group || scoped_to_group_database {
            token.revoked = true;
            token.revoked_at = Some(now);
            revoked += 1;
        }
    }
    if revoked > 0 {
        save_tokens(data_dir, &meta)?;
    }
    Ok(revoked)
}

pub fn ensure_phase8_token_metadata(data_dir: &Path) -> anyhow::Result<()> {
    let meta = load_tokens(data_dir)?;
    save_tokens(data_dir, &meta)
}
