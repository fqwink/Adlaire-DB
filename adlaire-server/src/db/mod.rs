pub mod manager;
pub mod meta;
pub mod sqld_adapter;

pub use manager::DbManager;

use crate::error::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbInfo {
    pub id:         String, // UUID v4
    pub name:       String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}

/// DB 名バリデーション（§6.4）
/// 正規表現: ^[a-zA-Z0-9_-]{1,127}$
/// 予約語: meta / admin（DB_RESERVED_NAME）
/// ___ を含む名前はブランチセパレータと衝突するため禁止（DB_RESERVED_NAME）
pub fn validate_db_name(name: &str) -> Result<(), AppError> {
    use std::sync::LazyLock;
    use regex::Regex;
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^[a-zA-Z0-9_-]{1,127}$").unwrap()
    });

    if !RE.is_match(name) {
        return Err(AppError::InvalidDbName);
    }
    if matches!(name, "meta" | "admin") || name.contains("___") {
        return Err(AppError::DbReservedName);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_db_name;
    use crate::error::AppError;

    #[test]
    fn accepts_valid_db_names() {
        for name in ["default", "tenant_1", "tenant-2", "A123"] {
            assert!(validate_db_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_invalid_db_names() {
        for name in ["", "a b", "../evil", "evil/path", "日本語"] {
            assert!(matches!(validate_db_name(name), Err(AppError::InvalidDbName)), "{name}");
        }

        let long_name = "a".repeat(128);
        assert!(matches!(validate_db_name(&long_name), Err(AppError::InvalidDbName)));
    }

    #[test]
    fn rejects_reserved_db_names() {
        for name in ["meta", "admin", "main___feature"] {
            assert!(matches!(validate_db_name(name), Err(AppError::DbReservedName)), "{name}");
        }
    }
}
