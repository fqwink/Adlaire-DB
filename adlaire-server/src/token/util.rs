use std::{collections::HashMap, fs::OpenOptions, io::Write, path::Path};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::auth::AccessLevel;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenRecord {
    pub id:         String,
    pub access:     AccessLevel,
    pub dbs:        Option<HashMap<String, AccessLevel>>,
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
