### Phase 2：データディレクトリ・libsql 統合

**目標**：データディレクトリを初期化し、libsql でシングル DB を開ける状態にする

**スコープ：**
- `--data` パスのディレクトリ作成・パーミッション設定
- flock による排他プロセスロック
- libsql::Builder::new_local() による DB オープン・WAL モード設定

**実装タスク：**

```
T-3: データディレクトリ初期化
  [ ] --data パスの作成（mkdir -p）
  [ ] .lock ファイルによる排他ロック（flock）
  [ ] databases/ meta/ サブディレクトリ作成
  [ ] ディレクトリパーミッション警告（700 未満で WARN）
  参照: §3.2, §8.1 Step 3〜4, §10.3

T-4: libsql 統合・DB オープン
  [ ] libsql::Builder::new_local() でシングル DB を開く
  [ ] PRAGMA journal_mode = WAL を起動時に適用
  [ ] busy_timeout を設定
  [ ] PRAGMA synchronous = NORMAL を設定
  [ ] サーバーシャットダウン時に Arc<libsql::Database> を drop（WAL flush + close）
  参照: §3.3.2, §13, §8.1 Step 6, §8.2 Step 3
  検証: TC-5（データ永続性）
```

---


#### 実装詳細

#### 14.14 DataDir・ProcessLock 実装

```rust
// data_dir.rs

impl DataDir {
    pub fn init(data_dir: &Path) -> anyhow::Result<()> {
        for sub in &["", "databases", "meta"] {
            let p = if sub.is_empty() { data_dir.to_path_buf() } else { data_dir.join(sub) };
            std::fs::create_dir_all(&p)?;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700))?;

            let mode = std::fs::metadata(&p)?.permissions().mode() & 0o777;
            if mode > 0o700 {
                tracing::warn!(
                    path = %p.display(),
                    mode = format!("{:04o}", mode),
                    "data directory permissions are broader than 0700"
                );
            }
        }
        // §8.1 Step 5-1: databases.json が存在しなければ空で初期化する
        let databases_path = data_dir.join("meta").join("databases.json");
        if !databases_path.exists() {
            std::fs::write(&databases_path, r#"{"databases":[]}"#)?;
        }

        // §8.1 Step 5-2: tokens.json が存在しなければ空で初期化する
        let tokens_path = data_dir.join("meta").join("tokens.json");
        if !tokens_path.exists() {
            std::fs::write(&tokens_path, r#"{"tokens":[]}"#)?;
        }

        tracing::info!(path = %data_dir.display(), "DataDir initialized");
        Ok(())
    }
}

// プロセス多重起動防止
pub struct ProcessLock {
    _file: std::fs::File,  // Drop 時に flock が自動解放される
}

impl ProcessLock {
    pub fn acquire(data_dir: &Path) -> anyhow::Result<Self> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .create(true).write(true)
            .open(data_dir.join(".lock"))?;
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            anyhow::bail!("another adlaire-db process is already running in {:?}", data_dir);
        }
        Ok(Self { _file: file })
    }
}
```

---


#### 14.19 SqldAdapter トレイト（db/sqld_adapter.rs）

libsql への依存を 1 ファイルに集約し、アップストリーム変更の影響範囲を限定する。

```rust
// db/sqld_adapter.rs

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
    /// 新規 Connection を生成して返す。
    /// Phase 9 の WebSocket セッション（WsStream）が永続コネクションとして保持するために使用する。
    fn connect(&self) -> Result<libsql::Connection, AppError>;

    /// 単一ステートメントを実行する。
    /// **注記**: RealSqldAdapter の実装では呼び出しのたびに新規 Connection を生成する。
    /// そのため、execute() を複数回呼び出しても同一トランザクション内に収まる保証はない。
    /// インタラクティブトランザクション（BEGIN/COMMIT を跨ぐ操作）は WsStream の conn フィールド経由で行う。
    async fn execute(
        &self,
        sql:       &str,
        args:      Vec<SqlValue>,
        want_rows: bool,
    ) -> Result<SqlResult, AppError>;

    /// 複数ステートメントを一括実行する（Sequence リクエスト用）
    async fn execute_batch(&self, sql: &str) -> Result<(), AppError>;
}

// ── RealSqldAdapter（libsql embedded） ────────────────────────────────────────

pub struct RealSqldAdapter {
    db: Arc<libsql::Database>,
}

impl RealSqldAdapter {
    pub async fn open(path: &Path, busy_timeout_ms: u64, run_integrity_check: bool) -> anyhow::Result<Self> {
        let db = libsql::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        let _ = conn.query("PRAGMA journal_mode=WAL", ()).await?;
        conn.execute("PRAGMA synchronous=NORMAL", ()).await?;
        let _ = conn.query(&format!("PRAGMA busy_timeout={busy_timeout_ms}"), ()).await?;

        if run_integrity_check {
            let mut rows = conn.query("PRAGMA integrity_check", ()).await?;
            let row = rows.next().await?
                .ok_or_else(|| anyhow::anyhow!("integrity_check returned no rows"))?;
            let val: String = row.get(0)?;
            if val != "ok" {
                anyhow::bail!("PRAGMA integrity_check failed for {:?}: {val}", path);
            }
            tracing::debug!(path = %path.display(), "integrity check passed");
        }

        tracing::debug!(path = %path.display(), "opened SQLite (WAL mode)");
        Ok(Self { db: Arc::new(db) })
    }
}

fn libsql_err(e: libsql::Error) -> AppError {
    let msg = e.to_string();
    if msg.contains("locked") || msg.contains("busy") {
        AppError::StorageBusy
    } else {
        AppError::Sqld(msg)
    }
}

#[async_trait::async_trait]
impl SqldAdapter for RealSqldAdapter {
    fn connect(&self) -> Result<libsql::Connection, AppError> {
        self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))
    }

    async fn execute_batch(&self, sql: &str) -> Result<(), AppError> {
        let conn = self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))?;
        conn.execute_batch(sql).await.map(|_| ()).map_err(libsql_err)
    }

    async fn execute(&self, sql: &str, args: Vec<SqlValue>, want_rows: bool) -> Result<SqlResult, AppError> {
        let conn   = self.db.connect().map_err(|e| AppError::Sqld(e.to_string()))?;
        let params = to_libsql_params(args);

        if want_rows {
            let mut rows = conn.query(sql, params).await.map_err(libsql_err)?;
            let col_count = rows.column_count();
            let cols: Vec<(Option<String>, Option<String>)> = (0..col_count)
                .map(|i| (rows.column_name(i).map(|s| s.to_string()), None))
                .collect();
            let mut result_rows: Vec<Vec<SqlValue>> = vec![];
            while let Some(row) = rows.next().await.map_err(libsql_err)? {
                let cells = (0..col_count)
                    .map(|i| from_libsql_value(row.get_value(i).unwrap_or(libsql::Value::Null)))
                    .collect();
                result_rows.push(cells);
            }
            Ok(SqlResult { cols, rows: result_rows, rows_affected: 0, last_insert_rowid: None })
        } else {
            let rows_affected      = conn.execute(sql, params).await.map_err(libsql_err)?;
            let last_insert_rowid  = conn.last_insert_rowid();
            Ok(SqlResult { cols: vec![], rows: vec![], rows_affected, last_insert_rowid: Some(last_insert_rowid) })
        }
    }
}

pub(crate) fn to_libsql_params(args: Vec<SqlValue>) -> Vec<libsql::Value> {
    args.into_iter().map(|v| match v {
        SqlValue::Null       => libsql::Value::Null,
        SqlValue::Integer(n) => libsql::Value::Integer(n),
        SqlValue::Real(f)    => libsql::Value::Real(f),
        SqlValue::Text(s)    => libsql::Value::Text(s),
        SqlValue::Blob(b)    => libsql::Value::Blob(b),
    }).collect()
}

pub(crate) fn from_libsql_value(v: libsql::Value) -> SqlValue {
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
    fn connect(&self) -> Result<libsql::Connection, AppError> {
        unimplemented!("MockSqldAdapter::connect")
    }
    async fn execute_batch(&self, _sql: &str) -> Result<(), AppError> { Ok(()) }
    async fn execute(&self, _sql: &str, _args: Vec<SqlValue>, _want_rows: bool) -> Result<SqlResult, AppError> {
        Ok(SqlResult::default())
    }
}
```

> **設計メモ**: `db/manager.rs` の `DbManager` は `Arc<dyn SqldAdapter>` を保持する。
> テストでは `MockSqldAdapter` を差し込んで libsql バイナリなしで単体テストが可能になる。
