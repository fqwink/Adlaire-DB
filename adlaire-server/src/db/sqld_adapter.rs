use crate::error::AppError;

#[derive(Debug, Default)]
pub struct SqlResult {
    pub cols:              Vec<String>,
    pub rows:              Vec<Vec<serde_json::Value>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<i64>,
}

/// sqld::Database の薄いラッパートレイト。
/// Phase 3 以降で RealSqldAdapter に差し替え、テストでは MockSqldAdapter を使用する。
#[async_trait::async_trait]
pub trait SqldAdapter: Send + Sync {
    async fn execute_raw(
        &self,
        sql:       &str,
        args:      Vec<serde_json::Value>,
        want_rows: bool,
    ) -> Result<SqlResult, AppError>;
}

// ── Mock（テスト・Phase 2 用） ──────────────────────────────────────────────────

pub struct MockSqldAdapter;

#[async_trait::async_trait]
impl SqldAdapter for MockSqldAdapter {
    async fn execute_raw(
        &self,
        _sql:       &str,
        _args:      Vec<serde_json::Value>,
        _want_rows: bool,
    ) -> Result<SqlResult, AppError> {
        Ok(SqlResult::default())
    }
}
