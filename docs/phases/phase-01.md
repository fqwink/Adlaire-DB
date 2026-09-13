### Phase 1：ビルド基盤・CLI

**目標**：Cargo ワークスペースを確立し、CLI の骨格を動かす

**スコープ：**
- Cargo workspace 初期化（adlaire-server crate + libsql crate）
- clap による `serve` / `token` サブコマンド骨格
- config.toml 3-way マージ（CLI > TOML > デフォルト）
- CI: cargo build / cargo test が通る状態を維持

**実装タスク：**

```
T-1: リポジトリ・ビルド基盤
  [ ] Cargo workspace 初期化（adlaire-server crate）
  [ ] libsql = "0.6" を [workspace.dependencies] に追加
  [ ] cargo build が通ることを確認
  [ ] CI: cargo build / cargo test が通る状態を維持
  参照: §3.3.1

T-2: CLI フレームワーク
  [ ] clap による `serve` / `token` サブコマンドの骨格実装
  [ ] `serve` フラグ: --data, --port, --admin-port, --auth-jwt-secret,
      --auth-jwt-secret-file, --log-level, --busy-timeout, --shutdown-timeout
  [ ] config.toml 読み込み（フラグ > config.toml > デフォルト）
  [ ] --data 未指定時の起動エラー
  参照: §4.1, §4.2, §8.4
```

---


#### 実装詳細

#### 14.12 CLI 構造体

```rust
// cli.rs
#[derive(Parser)]
#[command(name = "adlaire-db", version, about = "Self-hosted libSQL-compatible DB server")]
pub struct Cli { #[command(subcommand)] pub command: CliCommand }

#[derive(Subcommand)]
pub enum CliCommand {
    Serve(ServeArgs),
    Token { #[command(subcommand)] cmd: TokenSubcommand },
}

#[derive(Subcommand)]
pub enum TokenSubcommand { Create(TokenCreateArgs) }

#[derive(Parser, Default)]
pub struct ServeArgs {
    #[arg(long, required = true)] pub data:                    PathBuf,
    #[arg(long)] pub port:                                     Option<u16>,
    #[arg(long)] pub admin_port:                               Option<u16>,
    #[arg(long)] pub config:                                   Option<PathBuf>,
    #[arg(long)] pub auth_jwt_secret:                          Option<String>,
    #[arg(long)] pub auth_jwt_secret_file:                     Option<PathBuf>,
    #[arg(long)] pub admin_auth_token:                         Option<String>,
    #[arg(long)] pub log_level:                                Option<String>,
    #[arg(long, default_value_t = false)] pub skip_integrity_check: bool,
    #[arg(long)] pub replication_write_mode:                   Option<String>,
    #[arg(long)] pub busy_timeout:                             Option<u64>,
    #[arg(long)] pub shutdown_timeout:                         Option<u64>,
    // Phase 11: レプリケーション設定
    #[arg(long)] pub role:                                     Option<String>,
    #[arg(long)] pub primary_port:                             Option<u16>,
    #[arg(long)] pub primary_url:                              Option<String>,
    #[arg(long)] pub replication_auth_token:                   Option<String>,
}

#[derive(Parser)]
pub struct TokenCreateArgs {
    #[arg(long, required = true)] pub secret: String,
    #[arg(long, default_value = "rw")] pub access: String,
    #[arg(long)] pub expiry: Option<String>,
    #[arg(long, value_name = "DB:ACCESS")] pub db: Vec<String>,
}
```

`--data` のみ required。その他はすべて `Option<T>` にして 3-way マージで解決する。

---


#### 14.5 エントリポイント（main.rs）

CLI 構造体（`Cli`, `ServeArgs`, `TokenCreateArgs` 等）の定義は §14.12 を参照。

```rust
// main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Serve(args) => run_serve(args).await,
        CliCommand::Token { cmd: TokenSubcommand::Create(args) } => run_token_create(args),
    }
}

async fn run_serve(args: ServeArgs) -> anyhow::Result<()> {
    // Step 1: 設定マージ（CLI > config.toml > デフォルト）
    let config = Config::resolve(&args)?;

    // Step 2: ログ初期化（tracing + tracing-subscriber JSON）
    init_tracing(&config.log_level);

    tracing::info!(
        version  = env!("CARGO_PKG_VERSION"),
        data_dir = %config.data_dir.display(),
        port     = config.port,
        "Adlaire DB starting"
    );

    if config.storage.skip_integrity_check {
        tracing::warn!("--skip-integrity-check is set; startup integrity check disabled");
    }

    // Step 3: データディレクトリ初期化
    DataDir::init(&config.data_dir)?;

    // Step 4: プロセス排他ロック（flock LOCK_EX | LOCK_NB）
    let _lock = ProcessLock::acquire(&config.data_dir)?;

    // Step 5: AuthState 初期化（Phase 3 スタブ。Phase 4 で tokens.json 読み込みを追加）
    let auth = Arc::new(AuthState::new(&config));
    if !auth.is_auth_enabled() {
        tracing::warn!("JWT auth is disabled — all requests are unauthenticated");
    }
    // Phase 4 未実装のため JWT 設定時は起動を拒否する（verify() が常に失敗するため）
    anyhow::ensure!(
        !auth.is_auth_enabled(),
        "JWT auth is configured but not yet implemented (Phase 4). \
         Unset jwt_secret to start in unauthenticated mode."
    );

    // Step 6: DB 全件オープン（起動時整合性チェック込み）
    let db_mgr = Arc::new(
        DbManager::open_all(&config.data_dir, Arc::new(config.storage.clone())).await?
    );

    // Step 7: AppState 構築
    let role = parse_server_role(&args)?;
    let state: SharedState = Arc::new(AppState {
        config:      Arc::clone(&config),
        db_mgr,
        auth,
        metrics:     Arc::new(Metrics::new()),
        role,
        replication: None,
    });

    // Step 8: TCP ソケット bind
    let api_listener   = tokio::net::TcpListener::bind(("0.0.0.0",       config.port)).await?;
    let admin_listener = tokio::net::TcpListener::bind(("127.0.0.1", config.admin_port)).await?;

    tracing::info!(
        addr       = format!("0.0.0.0:{}", config.port),
        admin_addr = format!("127.0.0.1:{}", config.admin_port),
        "Adlaire DB listening"
    );

    // Step 9: グレースフルシャットダウン付きでサーバー起動
    let shutdown_timeout = config.shutdown_timeout;

    let (sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
    let mut sd_rx2 = sd_rx.clone();

    let api_task = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                tokio::select! {
                    Ok((stream, _)) = api_listener.accept() => {
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    TokioIo::new(stream),
                                    service_fn(move |req| {
                                        let state = Arc::clone(&state);
                                        async move { http::route(req, state).await }
                                    }),
                                )
                                .await;
                        });
                    }
                    _ = sd_rx.changed() => break,
                }
            }
        }
    });
    let admin_task = tokio::spawn({
        let state = Arc::clone(&state);
        async move {
            loop {
                tokio::select! {
                    Ok((stream, _)) = admin_listener.accept() => {
                        let state = Arc::clone(&state);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(
                                    TokioIo::new(stream),
                                    service_fn(move |req| {
                                        let state = Arc::clone(&state);
                                        async move { http::admin_route(req, state).await }
                                    }),
                                )
                                .await;
                        });
                    }
                    _ = sd_rx2.changed() => break,
                }
            }
        }
    });

    let sig_name = shutdown_signal_named().await;
    tracing::info!(signal = sig_name, "shutdown signal received");
    let _ = sd_tx.send(true);

    tokio::select! {
        _ = async { let _ = tokio::join!(api_task, admin_task); } => {},
        _ = tokio::time::sleep(std::time::Duration::from_secs(shutdown_timeout)) => {
            tracing::warn!(timeout_secs = shutdown_timeout, "graceful shutdown timed out, forcing exit");
        }
    }

    // Step 10: DB クローズ（WAL flush + checkpoint）
    match Arc::try_unwrap(state) {
        Ok(s) => match Arc::try_unwrap(s.db_mgr) {
            Ok(mgr) => mgr.close_all().await,
            Err(arc) => tracing::warn!(
                refs = Arc::strong_count(&arc),
                "db_mgr still referenced at shutdown — skipping close_all"
            ),
        },
        Err(arc) => tracing::warn!(
            refs = Arc::strong_count(&arc),
            "state still referenced at shutdown — skipping close_all"
        ),
    }

    tracing::info!("Adlaire DB stopped");
    Ok(())
}

fn run_token_create(args: TokenCreateArgs) -> anyhow::Result<()> {
    let secret = args.secret.as_bytes().to_vec();
    anyhow::ensure!(secret.len() >= 32, "--secret は 32 バイト以上の文字列を指定してください");

    let _access: AccessLevel = match args.access.as_str() {
        "rw" => AccessLevel::Rw,
        "ro" => AccessLevel::Ro,
        other => anyhow::bail!("unknown access level: {other}. Use 'rw' or 'ro'"),
    };

    // Phase 4 で JWT 発行を実装する
    println!("(Phase 4 stub) token create — secret len={}", secret.len());
    Ok(())
}

async fn shutdown_signal_named() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint  = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv()  => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    }
}

fn init_tracing(log_level: &str) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_new(log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .json()
        .with_env_filter(filter)
        .with_current_span(false)
        .init();
}
```


#### 14.13 Config 解決ロジック

> **フィールドマッピング注意**：`busy_timeout_ms` は TOML では `[server]` セクションに書くが（§4.2）、Rust の型システムでは `StorageConfig::busy_timeout_ms` に格納する。これは「ストレージに近い設定」として内部的に分類しているためで、TOML の `[storage]` に書いても無視される。

```rust
// config.rs

// TOML 構造体はすべてのフィールドを Option<T> にする
#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlConfig {
    pub server:      Option<TomlServer>,
    pub auth:        Option<TomlAuth>,
    pub admin:       Option<TomlAdmin>,
    pub storage:     Option<TomlStorage>,
    pub replication: Option<TomlReplication>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlServer {
    pub port:             Option<u16>,
    pub admin_port:       Option<u16>,
    pub log_level:        Option<String>,
    pub busy_timeout_ms:  Option<u64>,
    pub shutdown_timeout: Option<u64>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlAuth {
    pub jwt_secret:      Option<String>,
    pub jwt_secret_file: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlAdmin {
    pub auth_token: Option<String>,  // None = 認証無効（開発用）
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlStorage {
    pub wal_mode:                          Option<String>,
    pub skip_integrity_check:              Option<bool>,
    pub wal_retention_days:                Option<u64>,
    pub integrity_check_interval_hours:    Option<u64>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlReplication {
    pub write_mode: Option<String>,
}

impl Config {
    /// tokens.json のパスを返す
    pub fn tokens_path(&self) -> std::path::PathBuf {
        self.data_dir.join("meta").join("tokens.json")
    }

    pub fn resolve(args: &ServeArgs) -> anyhow::Result<Arc<Config>> {
        // 1. config.toml 読み込み（存在しなければ Default）
        let toml_path = args.config.clone()
            .unwrap_or_else(|| args.data.join("config.toml"));
        let toml: TomlConfig = if toml_path.exists() {
            let s = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&s)?
        } else {
            TomlConfig::default()
        };
        let srv  = toml.server.unwrap_or_default();
        let auth = toml.auth.unwrap_or_default();
        let adm  = toml.admin.unwrap_or_default();   // auth_token のみ
        let sto  = toml.storage.unwrap_or_default();
        let rep  = toml.replication.unwrap_or_default();

        // 2. JWT シークレット解決（優先度順）
        //    --auth-jwt-secret-file > --auth-jwt-secret
        //    > ADLAIRE_JWT_SECRET 環境変数
        //    > toml jwt_secret > toml jwt_secret_file
        let raw_secret: Option<Vec<u8>> = if let Some(p) = &args.auth_jwt_secret_file {
            Some(std::fs::read(p)?)
        } else if let Some(s) = &args.auth_jwt_secret {
            Some(s.as_bytes().to_vec())
        } else if let Ok(s) = std::env::var("ADLAIRE_JWT_SECRET") {
            Some(s.into_bytes())
        } else if let Some(s) = &auth.jwt_secret {
            Some(s.as_bytes().to_vec())
        } else if let Some(p) = &auth.jwt_secret_file {
            Some(std::fs::read(p)?)
        } else {
            None
        };

        // 3. シークレット長チェック（32 バイト未満は拒否）
        if let Some(ref b) = raw_secret {
            anyhow::ensure!(b.len() >= 32, "JWT secret must be at least 32 bytes");
        }

        // 4. 3-way マージ（CLI > TOML > デフォルト）
        let port       = args.port.or(srv.port).unwrap_or(8080);
        let admin_port = args.admin_port.or(srv.admin_port).unwrap_or(8081);
        let log_level_env = std::env::var("ADLAIRE_LOG_LEVEL").ok();
        let log_level  = args.log_level.as_deref()
            .or(log_level_env.as_deref())    // env（§8.4: CLI > env > TOML）
            .or(srv.log_level.as_deref())
            .unwrap_or("info")
            .to_string();
        let busy_timeout_ms: u64  = args.busy_timeout.or(srv.busy_timeout_ms).unwrap_or(5000);
        let shutdown_timeout: u64 = args.shutdown_timeout.or(srv.shutdown_timeout).unwrap_or(30);
        let skip_integrity_check = args.skip_integrity_check
            || sto.skip_integrity_check.unwrap_or(false);
        let wal_mode  = parse_wal_mode(sto.wal_mode.as_deref())?;
        let write_mode = match &args.replication_write_mode {
            Some(s) => parse_write_mode(s)?,
            None    => rep.write_mode.as_deref()
                           .map(parse_write_mode)
                           .transpose()?
                           .unwrap_or(ReplicationWriteMode::Async),
        };

        let admin_auth_token = args.admin_auth_token.clone()
            .or_else(|| std::env::var("ADLAIRE_ADMIN_TOKEN").ok())
            .or(adm.auth_token);

        Ok(Arc::new(Config {
            data_dir:             args.data.clone(),
            port,
            admin_port,
            log_level,
            admin_auth_token,
            jwt_secret_bytes:     raw_secret,
            shutdown_timeout,
            storage: StorageConfig {
                busy_timeout_ms:              busy_timeout_ms,
                wal_checkpoint_pages:         1000,
                wal_checkpoint_mode:          wal_mode,
                wal_retention_days:           0,
                integrity_check_interval_hrs: 0,
                skip_integrity_check,
            },
            replication: ReplicationConfig {
                write_mode,
                sync_timeout_ms: 5000,
                auth_token: args.replication_auth_token.clone(),
            },
        }))
    }
}

fn parse_wal_mode(s: Option<&str>) -> anyhow::Result<WalCheckpointMode> {
    match s.unwrap_or("passive") {
        "passive" => Ok(WalCheckpointMode::Passive),
        "full"    => Ok(WalCheckpointMode::Full),
        "restart" => Ok(WalCheckpointMode::Restart),
        other     => anyhow::bail!("unknown wal_mode: {other}. Use 'passive', 'full', or 'restart'"),
    }
}

fn parse_write_mode(s: &str) -> anyhow::Result<ReplicationWriteMode> {
    match s {
        "async" => Ok(ReplicationWriteMode::Async),
        "sync"  => Ok(ReplicationWriteMode::Sync),
        other   => anyhow::bail!("unknown replication write_mode: {other}. Use 'async' or 'sync'"),
    }
}

pub fn parse_server_role(args: &ServeArgs) -> anyhow::Result<ServerRole> {
    match args.role.as_deref().unwrap_or("standalone") {
        "standalone" => Ok(ServerRole::Standalone),
        "primary"    => Ok(ServerRole::Primary {
            primary_port: args.primary_port.unwrap_or(8082),
        }),
        // Phase 11 で Replica variant 解除後に有効化：
        // "replica" => {
        //     let url = args.primary_url.as_deref()
        //         .ok_or_else(|| anyhow::anyhow!("--primary-url は --role replica 時に必須です"))?;
        //     Ok(ServerRole::Replica { primary_url: url.parse()? })
        // }
        other => anyhow::bail!(
            "unknown role '{other}'. Use 'standalone', 'primary', or 'replica'"
        ),
    }
}
```

---
