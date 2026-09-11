use std::{path::Path, sync::Arc};

use crate::error::AppError;

// ── SQL 値型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

// ── 実行結果 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SqlResult {
    pub cols:              Vec<(Option<String>, Option<String>)>, // (name, decltype)
    pub rows:              Vec<Vec<SqlValue>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<i64>,
}

// ── SqldAdapter トレイト ──────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait SqldAdapter: Send + Sync {
    async fn execute(
        &self,
        sql:       &str,
        args:      Vec<SqlValue>,
        want_rows: bool,
    ) -> Result<SqlResult, AppError>;
}

// ── RealSqldAdapter（libsql embedded） ────────────────────────────────────────

pub struct RealSqldAdapter {
    db: Arc<libsql::Database>,
}

impl RealSqldAdapter {
    pub async fn open(path: &Path, busy_timeout_ms: u64) -> anyhow::Result<Self> {
        let db = libsql::Builder::new_local(path)
            .build()
            .await?;

        let conn = db.connect()?;
        // journal_mode と busy_timeout は結果行を返すため query() を使う
        let _ = conn.query("PRAGMA journal_mode=WAL", ()).await?;
        conn.execute("PRAGMA synchronous=NORMAL", ()).await?;
        let _ = conn.query(
            &format!("PRAGMA busy_timeout={busy_timeout_ms}"),
            (),
        ).await?;

        tracing::debug!(path = %path.display(), "opened SQLite (WAL mode)");
        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait::async_trait]
impl SqldAdapter for RealSqldAdapter {
    async fn execute(
        &self,
        sql:       &str,
        args:      Vec<SqlValue>,
        want_rows: bool,
    ) -> Result<SqlResult, AppError> {
        let conn   = self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))?;
        let params = to_libsql_params(args);

        if want_rows {
            let mut rows = conn
                .query(sql, params)
                .await
                .map_err(|e| AppError::Sqld(e.to_string()))?;

            let col_count = rows.column_count();
            let cols: Vec<(Option<String>, Option<String>)> = (0..col_count)
                .map(|i| {
                    let name = rows.column_name(i).map(|s| s.to_string());
                    (name, None) // decltype not exposed by libsql yet
                })
                .collect();

            let mut result_rows: Vec<Vec<SqlValue>> = vec![];
            while let Some(row) = rows.next().await.map_err(|e| AppError::Sqld(e.to_string()))? {
                let cells = (0..col_count)
                    .map(|i| {
                        let v = row.get_value(i).unwrap_or(libsql::Value::Null);
                        from_libsql_value(v)
                    })
                    .collect();
                result_rows.push(cells);
            }

            Ok(SqlResult {
                cols,
                rows:              result_rows,
                rows_affected:     0,
                last_insert_rowid: None,
            })
        } else {
            let rows_affected = conn
                .execute(sql, params)
                .await
                .map_err(|e| AppError::Sqld(e.to_string()))?;

            let last_insert_rowid = conn.last_insert_rowid();

            Ok(SqlResult {
                cols:              vec![],
                rows:              vec![],
                rows_affected,
                last_insert_rowid: if last_insert_rowid != 0 { Some(last_insert_rowid) } else { None },
            })
        }
    }
}

fn to_libsql_params(args: Vec<SqlValue>) -> Vec<libsql::Value> {
    args.into_iter()
        .map(|v| match v {
            SqlValue::Null       => libsql::Value::Null,
            SqlValue::Integer(n) => libsql::Value::Integer(n),
            SqlValue::Real(f)    => libsql::Value::Real(f),
            SqlValue::Text(s)    => libsql::Value::Text(s),
            SqlValue::Blob(b)    => libsql::Value::Blob(b),
        })
        .collect()
}

fn from_libsql_value(v: libsql::Value) -> SqlValue {
    match v {
        libsql::Value::Null       => SqlValue::Null,
        libsql::Value::Integer(n) => SqlValue::Integer(n),
        libsql::Value::Real(f)    => SqlValue::Real(f),
        libsql::Value::Text(s)    => SqlValue::Text(s),
        libsql::Value::Blob(b)    => SqlValue::Blob(b),
    }
}

// ── MockSqldAdapter（テスト用） ───────────────────────────────────────────────

pub struct MockSqldAdapter;

#[async_trait::async_trait]
impl SqldAdapter for MockSqldAdapter {
    async fn execute(
        &self,
        _sql:       &str,
        _args:      Vec<SqlValue>,
        _want_rows: bool,
    ) -> Result<SqlResult, AppError> {
        Ok(SqlResult::default())
    }
}
