mod auth;
mod cli;
mod config;
mod data_dir;
mod db;
mod error;
mod hrana;
mod http;
mod metrics;
mod state;

use std::sync::Arc;

use clap::Parser;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

use crate::{
    auth::{AuthState, AccessLevel},
    cli::{Cli, CliCommand, TokenSubcommand},
    config::Config,
    data_dir::{DataDir, ProcessLock},
    db::DbManager,
    http::{admin_route, route},
    metrics::Metrics,
    state::{AppState, ServerRole},
};

// ── エントリポイント ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Serve(args)                                    => run_serve(args).await,
        CliCommand::Token { cmd: TokenSubcommand::Create(args) }   => run_token_create(args),
    }
}

// ── serve ────────────────────────────────────────────────────────────────────

async fn run_serve(args: crate::cli::ServeArgs) -> anyhow::Result<()> {
    let config = Config::resolve(&args)?;

    init_tracing(&config.log_level);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        data_dir = %config.data_dir.display(),
        port = config.port,
        "Adlaire DB starting"
    );

    // G: integrity check スキップ時の警告
    if config.skip_integrity_check {
        tracing::warn!("--skip-integrity-check is set; startup integrity check disabled");
    }

    // Step 3: データディレクトリ初期化
    DataDir::init(&config.data_dir)?;

    // Step 4: プロセス排他ロック
    let _lock = ProcessLock::acquire(&config.data_dir)?;

    // Step 5: AuthState 初期化
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

    // Step 6: DB 全件オープン
    let db_mgr = Arc::new(
        DbManager::open_all(&config.data_dir, Arc::new(config.storage.clone())).await?
    );

    // Step 7: AppState 構築
    let state: crate::state::SharedState = Arc::new(AppState {
        config:      Arc::clone(&config),
        db_mgr,
        auth,
        metrics:     Arc::new(Metrics::new()),
        role:        ServerRole::Standalone,
        replication: None,
    });

    // Step 8: TCP ソケット bind
    let api_listener   = tokio::net::TcpListener::bind(("0.0.0.0",       config.port)).await?;
    let admin_listener = tokio::net::TcpListener::bind(("127.0.0.1", config.admin_port)).await?;

    tracing::info!(
        addr = format!("0.0.0.0:{}", config.port),
        admin_addr = format!("127.0.0.1:{}", config.admin_port),
        "Adlaire DB listening"
    );

    // Step 9: グレースフルシャットダウン付きでサーバー起動（I）
    let shutdown_timeout = config.shutdown_timeout;

    let (sd_tx, mut sd_rx) = tokio::sync::watch::channel(false);
    let mut sd_rx2 = sd_rx.clone();

    let api_state = Arc::clone(&state);
    let api_task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = api_listener.accept() => {
                    let (stream, _) = match accepted {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(err = %e, "API listener accept failed");
                            continue;
                        }
                    };
                    let state = Arc::clone(&api_state);
                    connections.spawn(async move {
                        let io = TokioIo::new(stream);
                        let service = service_fn(move |req| {
                            let state = Arc::clone(&state);
                            route(req, state)
                        });
                        if let Err(e) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            tracing::debug!(err = %e, "API connection closed with error");
                        }
                    });
                }
                _ = sd_rx.changed() => break,
            }
        }
        while let Some(result) = connections.join_next().await {
            if let Err(e) = result {
                tracing::debug!(err = %e, "API connection task failed");
            }
        }
    });

    let admin_state = Arc::clone(&state);
    let admin_task = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted = admin_listener.accept() => {
                    let (stream, _) = match accepted {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(err = %e, "admin listener accept failed");
                            continue;
                        }
                    };
                    let state = Arc::clone(&admin_state);
                    connections.spawn(async move {
                        let io = TokioIo::new(stream);
                        let service = service_fn(move |req| {
                            let state = Arc::clone(&state);
                            admin_route(req, state)
                        });
                        if let Err(e) = http1::Builder::new()
                            .serve_connection(io, service)
                            .await
                        {
                            tracing::debug!(err = %e, "admin connection closed with error");
                        }
                    });
                }
                _ = sd_rx2.changed() => break,
            }
        }
        while let Some(result) = connections.join_next().await {
            if let Err(e) = result {
                tracing::debug!(err = %e, "admin connection task failed");
            }
        }
    });

    // K: シグナル名付きシャットダウンログ
    let sig_name = shutdown_signal_named().await;
    tracing::info!(signal = sig_name, "shutdown signal received");
    let _ = sd_tx.send(true);

    let api_abort = api_task.abort_handle();
    let admin_abort = admin_task.abort_handle();
    if tokio::time::timeout(
        std::time::Duration::from_secs(shutdown_timeout),
        async { let _ = tokio::join!(api_task, admin_task); },
    )
    .await
    .is_err()
    {
        tracing::warn!(timeout_secs = shutdown_timeout, "graceful shutdown timed out, forcing exit");
        api_abort.abort();
        admin_abort.abort();
        tokio::task::yield_now().await;
    }

    // Step 10: DB クローズ
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

// ── token create ─────────────────────────────────────────────────────────────

fn run_token_create(args: crate::cli::TokenCreateArgs) -> anyhow::Result<()> {
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

// ── tracing 初期化 ────────────────────────────────────────────────────────────

fn init_tracing(log_level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_new(log_level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();
}

// ── shutdown signal ───────────────────────────────────────────────────────────

async fn shutdown_signal_named() -> &'static str {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint  = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv()  => "SIGINT",
        _ = sigterm.recv() => "SIGTERM",
    }
}
