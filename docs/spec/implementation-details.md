# 共通実装詳細仕様

モジュール構成、主要型定義、共通実装詳細を固定する仕様である。

### 9.18 共通実装詳細

全フェーズで共有される型定義・モジュール構成を以下に示す。

> **§14.x 番号規則**：§14 以下のサブセクション番号は実装順ではなく参照便宜のための固定 ID。欠番（§14.x が存在しない番号）は将来追加のために予約している。各フェーズの実装詳細は対応フェーズ節の直下に配置するが、§14.x 番号で相互参照できる。

#### 14.1 モジュール構成

```
adlaire-server/src/
├── main.rs              ← エントリポイント・tokio ランタイム起動・run_serve / run_token_create
├── cli.rs               ← Cli / CliCommand / ServeArgs / TokenCreateArgs（clap derive）
├── config.rs            ← Config struct・CLI フラグと config.toml のマージ
├── data_dir.rs          ← DataDir::init()・ProcessLock::acquire()（flock）
├── error.rs             ← AppError enum（thiserror）・HTTP レスポンス変換
├── state.rs             ← AppState struct・Arc<> ラッパー定義
├── metrics.rs           ← Metrics struct（Phase 10 スタブ）
├── db/
│   ├── mod.rs           ← DB 名バリデーション・DbInfo 型
│   ├── manager.rs       ← DbManager struct・open/close/create/delete ロジック
│   ├── meta.rs          ← databases.json / tokens.json / branches.json 読み書き
│   └── sqld_adapter.rs  ← SqldAdapter トレイト・RealSqldAdapter 実装（§14.19）
├── auth/
│   ├── mod.rs           ← JWT 検証ロジック・Claims / AuthState struct
│   └── middleware.rs    ← extract_claims() 関数: JWT → Claims
├── token/
│   └── util.rs          ← parse_expiry() / generate_token_id()（Phase 4）
├── http/
│   ├── mod.rs           ← route() / admin_route() 手動ルーティング・hyper サーバー起動
│   ├── pipeline.rs      ← POST /v2/pipeline ハンドラ
│   ├── health.rs        ← GET /v2/health ハンドラ
│   └── admin/
│       └── mod.rs       ← 管理 API Router・admin_auth_middleware・各ハンドラスタブ（Phase 6〜14）
├── hrana/
│   ├── mod.rs           ← hrana-http v2 型の re-export
│   ├── types.rs         ← PipelineRequest / PipelineResponse / Value 等
│   └── convert.rs       ← libsql 行・カラム型 → hrana 型変換
├── ws/
│   ├── mod.rs           ← hrana-ws v3 WebSocket ハンドラ（Phase 9）
│   ├── session.rs       ← WsSession・stream_id ごとの状態管理
│   └── types.rs         ← ClientMsg / ServerMsg 型定義
├── replication/
│   ├── mod.rs           ← WAL レプリケーション共通型（Phase 11）
│   ├── primary.rs       ← SSE /replication/v1/log・snapshot ハンドラ
│   └── replica.rs       ← フレーム受信・CRC32 検証・適用ループ
└── wal/
    ├── mod.rs           ← WAL アーカイブ公開 API（Phase 13〜14）
    ├── archive.rs       ← フレーム書き込み・fsync・manifest 更新
    └── manifest.rs      ← Manifest / FrameMeta struct・アトミック保存
```


#### 14.2 主要型定義

#### AppState

```rust
// state.rs
#[derive(Clone)]
pub struct AppState {
    pub config:      Arc<Config>,
    pub db_mgr:      Arc<DbManager>,
    pub auth:        Arc<AuthState>,
    pub metrics:     Arc<Metrics>,                    // Phase 10～
    pub role:        ServerRole,                      // Phase 11～（デフォルト Standalone）
    pub replication: Option<Arc<()>>,  // Phase 11 で ReplicationState に差し替え
}

pub type SharedState = Arc<AppState>;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum ServerRole {
    Standalone,
    Primary { primary_port: u16 },
    // Phase 11: 下記を有効化（url クレート追加が前提）
    // Replica { primary_url: url::Url },
}
```

#### Config

```rust
// config.rs
#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir:          PathBuf,
    pub port:              u16,              // デフォルト 8080
    pub admin_port:        u16,              // デフォルト 8081
    pub log_level:         String,           // "trace" | "debug" | "info" | "warn" | "error"
    pub admin_auth_token:  Option<String>,   // None = 認証無効（開発用）
    pub jwt_secret_bytes:  Option<Vec<u8>>, // 32 バイト以上。None = 認証無効
    pub shutdown_timeout:  u64,             // グレースフルシャットダウン最大秒数（デフォルト 30）
    pub storage:           StorageConfig,   // skip_integrity_check は StorageConfig に移管
    pub replication:       ReplicationConfig,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub busy_timeout_ms:              u64,  // デフォルト 5000
    pub wal_checkpoint_pages:         u64,  // デフォルト 1000
    pub wal_checkpoint_mode:          WalCheckpointMode,
    pub wal_retention_days:           u64,  // 0 = PITR 無効
    pub integrity_check_interval_hrs: u64,  // 0 = 無効
    pub skip_integrity_check:         bool, // 起動時整合性チェックをスキップ（デフォルト false）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalCheckpointMode { Passive, Full, Restart }

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub write_mode:      ReplicationWriteMode,
    pub sync_timeout_ms: u64,           // デフォルト 5000
    pub auth_token:      Option<String>, // --replication-auth-token（Phase 11）
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationWriteMode { Async, Sync }
```

#### DbManager

```rust
// db/manager.rs
pub struct DbManager {
    data_dir: PathBuf,
    dbs:      tokio::sync::RwLock<HashMap<String, Arc<dyn SqldAdapter>>>,
    meta:     tokio::sync::RwLock<DatabasesMeta>,
    config:   Arc<StorageConfig>,
}

impl DbManager {
    /// 起動時: databases.json を読み込み、各 DB を RealSqldAdapter::open() でオープン
    /// Phase 3 シングル DB モード: "default" が存在しなければ自動作成する
    pub async fn open_all(data_dir: &Path, config: Arc<StorageConfig>) -> anyhow::Result<Self> {
        let mut meta = DatabasesMeta::load(data_dir)?;

        // "default" DB が存在しなければ作成する（Phase 3 シングル DB モード）
        if !meta.databases.iter().any(|d| d.name == "default") {
            let info = DbInfo {
                id:         uuid::Uuid::new_v4().to_string(),
                name:       "default".to_string(),
                created_at: chrono::Utc::now(),
                size_bytes: 0,
            };
            let db_path = data_dir.join("databases").join("default");
            std::fs::create_dir_all(&db_path)?;
            meta.databases.push(info);
            meta.save(data_dir)?;
        }

        let mut dbs: HashMap<String, Arc<dyn SqldAdapter>> = HashMap::new();
        for db in &meta.databases {
            let db_file = data_dir.join("databases").join(&db.name).join("data.db");
            match RealSqldAdapter::open(&db_file, config.busy_timeout_ms, !config.skip_integrity_check).await {
                Ok(adapter) => {
                    dbs.insert(db.name.clone(), Arc::new(adapter));
                    tracing::info!(db = %db.name, "opened database");
                }
                Err(e) => {
                    tracing::error!(db = %db.name, err = %e, "failed to open database");
                }
            }
        }

        tracing::info!(count = dbs.len(), "DbManager ready");
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            dbs:      tokio::sync::RwLock::new(dbs),
            meta:     tokio::sync::RwLock::new(meta),
            config,
        })
    }

    /// DB 名 → SqldAdapter を返す（存在しない場合 None）
    pub async fn get(&self, name: &str) -> Option<Arc<dyn SqldAdapter>> {
        self.dbs.read().await.get(name).cloned()
    }

    /// DB 作成: 存在チェック（読み取りロック）→ ディレクトリ作成 → アダプタ生成 → meta 更新（書き込みロック）
    pub async fn create(&self, name: &str) -> Result<DbInfo, AppError> {
        {
            let meta = self.meta.read().await;
            if meta.databases.iter().any(|d| d.name == name) {
                return Err(AppError::DbAlreadyExists(name.to_string()));
            }
        }

        let db_path = self.data_dir.join("databases").join(name);
        std::fs::create_dir_all(&db_path).map_err(|e| AppError::Internal(e.into()))?;

        let db_file = db_path.join("data.db");
        let adapter = RealSqldAdapter::open(&db_file, self.config.busy_timeout_ms, !self.config.skip_integrity_check)
            .await
            .map_err(|e| AppError::Internal(e))?;

        let info = DbInfo {
            id:         uuid::Uuid::new_v4().to_string(),
            name:       name.to_string(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
        };

        {
            let mut meta = self.meta.write().await;
            meta.databases.push(info.clone());
            meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        }

        self.dbs.write().await.insert(name.to_string(), Arc::new(adapter));
        tracing::info!(db = name, id = %info.id, "created database");
        Ok(info)
    }

    /// DB 削除: 存在チェック（読み取りロック）→ アダプタ削除 → ディレクトリ削除 → meta 更新（書き込みロック）
    pub async fn delete(&self, name: &str) -> Result<(), AppError> {
        {
            let meta = self.meta.read().await;
            if !meta.databases.iter().any(|d| d.name == name) {
                return Err(AppError::DbNotFound(name.to_string()));
            }
        }

        self.dbs.write().await.remove(name);

        let db_path = self.data_dir.join("databases").join(name);
        if db_path.exists() {
            std::fs::remove_dir_all(&db_path).map_err(|e| AppError::Internal(e.into()))?;
        }

        {
            let mut meta = self.meta.write().await;
            meta.databases.retain(|d| d.name != name);
            meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        }

        tracing::info!(db = name, "deleted database");
        Ok(())
    }

    /// DB 一覧（size_bytes は data.db のファイルサイズを動的取得）
    pub async fn list(&self) -> Vec<DbInfo> {
        let meta = self.meta.read().await;
        meta.databases.iter().map(|info| {
            let db_path = self.data_dir.join("databases").join(&info.name).join("data.db");
            let size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
            DbInfo { size_bytes, ..info.clone() }
        }).collect()
    }

    /// DB 名で DbInfo を返す（存在しない場合は DbNotFound、size_bytes はファイルから動的取得）
    pub async fn get_info(&self, name: &str) -> Result<DbInfo, AppError> {
        let meta = self.meta.read().await;
        let info = meta.databases.iter()
            .find(|d| d.name == name)
            .cloned()
            .ok_or_else(|| AppError::DbNotFound(name.to_string()))?;
        let db_path = self.data_dir.join("databases").join(name).join("data.db");
        let size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
        Ok(DbInfo { size_bytes, ..info })
    }

    /// シャットダウン時: ログのみ（Arc<dyn SqldAdapter> のドロップは Drop に委ねる）
    pub async fn close_all(self) {
        let count = self.dbs.read().await.len();
        tracing::info!(count, "closing all databases");
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbInfo {
    pub id:         String,  // UUID v4（作成時に uuid::Uuid::new_v4().to_string() で生成）
    pub name:       String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}
```

#### JWT Claims と AuthState

```rust
// auth/mod.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub iss: Option<String>,
    pub sub: String,                             // token_id（tok_xxx）
    pub iat: i64,
    pub exp: Option<i64>,
    pub a:   AccessLevel,
    pub dbs: Option<HashMap<String, AccessLevel>>, // Phase 7～
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessLevel { Rw, Ro }

impl Claims {
    /// DB 名に対するアクセスレベルを解決する
    pub fn resolve_access(&self, db_name: &str) -> AccessLevel {
        match &self.dbs {
            Some(dbs) => dbs.get(db_name).cloned().unwrap_or(self.a.clone()),
            None      => self.a.clone(),
        }
    }

    /// DB 名に対する書き込み権限の有無を返す（per-DB アクセスレベルを優先）
    pub fn can_write_db(&self, db_name: &str) -> bool {
        self.resolve_access(db_name) == AccessLevel::Rw
    }

    /// 認証無効モード用（jwt_secret 未設定時のみ使用）
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
}

/// Phase 3 スタブ。Phase 4 で full 実装（jsonwebtoken / revoke リスト）に差し替える。
pub struct AuthState {
    pub(crate) secret_bytes: Option<Vec<u8>>,
}

impl AuthState {
    /// 設定から生成する（Phase 3 スタブ）
    pub fn new(config: &Config) -> Self {
        Self { secret_bytes: config.jwt_secret_bytes.clone() }
    }

    /// JWT 認証が有効かどうか（secret が設定されていれば true）
    pub fn is_auth_enabled(&self) -> bool { self.secret_bytes.is_some() }

    /// JWT 検証（Phase 4 で実装。Phase 3 では常に AuthInvalid を返す）
    pub async fn verify(&self, _raw_token: &str) -> Result<Claims, AppError> {
        Err(AppError::AuthInvalid)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenRecord {
    pub id:         String,
    pub access:     AccessLevel,
    pub dbs:        Option<HashMap<String, AccessLevel>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked:    bool,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}
```

#### AppError（全エラーコード対応）

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    AuthRequired,
    #[error("invalid or revoked token")]
    AuthInvalid,
    #[error("token expired")]
    AuthExpired,
    #[error("permission denied")]
    PermissionDenied,
    #[error("database not found: {0}")]
    DbNotFound(String),
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("database already exists: {0}")]
    DbAlreadyExists(String),
    #[error("invalid database name")]
    InvalidDbName,
    #[error("reserved database name")]
    DbReservedName,
    #[error("invalid request")]
    InvalidRequest,
    #[error("not acceptable")]
    NotAcceptable,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("database is busy")]
    StorageBusy,
    #[error("replication timeout")]
    ReplicationTimeout,
    #[error("PITR not enabled")]
    PitrNotEnabled,
    #[error("frame not found")]
    FrameNotFound,
    #[error("restore integrity check failed")]
    RestoreIntegrityFailed,
    #[error("WAL frame corrupt")]
    RestoreFrameCorrupt,
    #[error("authentication is disabled")]
    AuthDisabled,
    #[error("config error: {0}")]
    ConfigError(String),
    #[error("sqld error: {0}")]
    Sqld(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn into_response(self) -> Response<Full<Bytes>> {
        use http::StatusCode;
        let (status, code) = match &self {
            Self::AuthRequired          => (StatusCode::UNAUTHORIZED,            "AUTH_REQUIRED"),
            Self::AuthInvalid           => (StatusCode::UNAUTHORIZED,            "AUTH_INVALID"),
            Self::AuthExpired           => (StatusCode::UNAUTHORIZED,            "AUTH_EXPIRED"),
            Self::PermissionDenied      => (StatusCode::FORBIDDEN,               "PERMISSION_DENIED"),
            Self::DbNotFound(_)         => (StatusCode::NOT_FOUND,               "DB_NOT_FOUND"),
            Self::TokenNotFound(_)      => (StatusCode::NOT_FOUND,               "TOKEN_NOT_FOUND"),
            Self::DbAlreadyExists(_)    => (StatusCode::CONFLICT,                "DB_ALREADY_EXISTS"),
            Self::InvalidDbName         => (StatusCode::BAD_REQUEST,             "INVALID_DB_NAME"),
            Self::DbReservedName        => (StatusCode::BAD_REQUEST,             "DB_RESERVED_NAME"),
            Self::InvalidRequest        => (StatusCode::BAD_REQUEST,             "INVALID_REQUEST"),
            Self::NotAcceptable         => (StatusCode::NOT_ACCEPTABLE,          "NOT_ACCEPTABLE"),
            Self::PayloadTooLarge       => (StatusCode::PAYLOAD_TOO_LARGE,       "PAYLOAD_TOO_LARGE"),
            Self::StorageBusy           => (StatusCode::SERVICE_UNAVAILABLE,     "STORAGE_BUSY"),
            Self::ReplicationTimeout    => (StatusCode::SERVICE_UNAVAILABLE,     "REPLICATION_TIMEOUT"),
            Self::PitrNotEnabled        => (StatusCode::SERVICE_UNAVAILABLE,     "PITR_NOT_ENABLED"),
            Self::FrameNotFound         => (StatusCode::NOT_FOUND,               "FRAME_NOT_FOUND"),
            Self::RestoreIntegrityFailed=> (StatusCode::CONFLICT,                "RESTORE_INTEGRITY_FAILED"),
            Self::RestoreFrameCorrupt   => (StatusCode::CONFLICT,                "RESTORE_FRAME_CORRUPT"),
            Self::AuthDisabled          => (StatusCode::UNAUTHORIZED,            "AUTH_DISABLED"),
            Self::ConfigError(_)        => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
            Self::Sqld(_)              => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
            Self::Internal(_)           => (StatusCode::INTERNAL_SERVER_ERROR,   "INTERNAL_ERROR"),
        };
        let body = serde_json::to_vec(&serde_json::json!({
            "error": self.to_string(),
            "code":  code,
        })).unwrap_or_default();
        Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(Full::from(body))
            .unwrap()
    }
}
```

#### hrana 型（hrana-http v2 完全定義）

```rust
// hrana/types.rs

// ─── リクエスト ───────────────────────────────────────────
#[derive(Debug, serde::Deserialize)]
pub struct PipelineRequest {
    pub baton:    Option<String>,
    pub requests: Vec<StreamRequest>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamRequest {
    Execute  { stmt: Stmt },
    Close,
    Sequence { sql: String },
}

#[derive(Debug, serde::Deserialize)]
pub struct Stmt {
    pub sql:        String,
    #[serde(default)]
    pub args:       Vec<Value>,
    #[serde(default)]
    pub named_args: Vec<NamedArg>,
    #[serde(default)]
    pub want_rows:  bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Value {
    Integer { value: String },     // 整数を文字列で表現（i64 の範囲）
    Real    { value: f64 },
    Text    { value: String },
    Blob    { value: String },     // base64 エンコード
    Null,
}

#[derive(Debug, serde::Deserialize)]
pub struct NamedArg {
    pub name:  String,
    pub value: Value,
}

// ─── レスポンス ──────────────────────────────────────────
#[derive(Debug, serde::Serialize)]
pub struct PipelineResponse {
    pub baton:    Option<String>,
    pub base_url: Option<String>,
    pub results:  Vec<StreamResult>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamResult {
    Ok    { response: StreamResponse },
    Error { error: HranaError },
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResponse {
    Execute  { result: StmtResult },
    Sequence,
    Close,
}

#[derive(Debug, serde::Serialize)]
pub struct StmtResult {
    pub cols:              Vec<Col>,
    pub rows:              Vec<Vec<Value>>,
    pub rows_affected:     u64,
    pub last_insert_rowid: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Col {
    pub name:     Option<String>,
    pub decltype: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct HranaError {
    pub message: String,
    pub code:    String,
}
```

#### WAL アーカイブ manifest 型（Phase 13〜14）

```rust
// wal/manifest.rs
#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Manifest {
    pub version:    u32,             // フォーマットバージョン = 1
    pub base_frame: u64,             // 最新スナップショット時点の WAL フレーム番号
    pub snapshot:   Option<String>,  // "snapshot-{frame_no:012}.db"
    pub frames:     Vec<FrameMeta>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameMeta {
    pub frame_no:   u64,
    pub file:       String,                       // "frame-{frame_no:012}.bin"
    pub size:       u64,
    pub checksum:   u32,                          // CRC32
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Manifest {
    pub async fn load(path: &std::path::Path) -> anyhow::Result<Self>;

    /// 一時ファイルへ書き込み → rename（POSIX アトミック）
    pub async fn save_atomic(&self, path: &std::path::Path) -> anyhow::Result<()>;

    /// timestamp 以前の全フレームを返す
    pub fn frames_before(&self, ts: chrono::DateTime<chrono::Utc>) -> Vec<&FrameMeta>;

    /// frame_no 以下の全フレームを返す
    pub fn frames_at_or_before(&self, frame_no: u64) -> Vec<&FrameMeta>;

    /// 保持期間切れフレームをリストから除去し、除去したメタデータを返す
    pub fn prune_before(&mut self, cutoff: chrono::DateTime<chrono::Utc>) -> Vec<FrameMeta>;
}
```

#### Metrics（Phase 10 スタブ）

```rust
// metrics.rs
// Phase 10 で AtomicU64 カウンター・DashMap に拡張する
#[derive(Default)]
pub struct Metrics;

impl Metrics {
    pub fn new() -> Self { Self }
}
```

Phase 10 実装時の完全版（参考）：
```rust
// Phase 10 で以下に差し替える
pub struct Metrics {
    pub started_at:      std::time::Instant,
    pub databases:       dashmap::DashMap<String, DbMetrics>,
    pub tokens_total:    std::sync::atomic::AtomicU64,
    pub tokens_revoked:  std::sync::atomic::AtomicU64,
}

#[derive(Default)]
pub struct DbMetrics {
    pub queries_total:      std::sync::atomic::AtomicU64,
    pub rows_read_total:    std::sync::atomic::AtomicU64,
    pub rows_written_total: std::sync::atomic::AtomicU64,
    pub connections_active: std::sync::atomic::AtomicI64,
    pub integrity_errors:   std::sync::atomic::AtomicU64,
    pub wal_size_bytes:     std::sync::atomic::AtomicU64,
}
```


**Phase 1〜19 の詳細：** `docs/spec/` に分割して管理する。HTML 生成時は以下の include を展開する。
