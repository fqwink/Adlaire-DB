mod auth;
mod cli;
mod config;
mod data_dir;
mod db;
mod error;

use std::sync::Arc;

use clap::Parser;

use crate::{
    auth::AccessLevel,
    cli::{Cli, CliCommand, TokenSubcommand},
    config::Config,
    data_dir::{DataDir, ProcessLock},
    db::DbManager,
};

// ── エントリポイント ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Serve(args) => run_serve(args).await,
        CliCommand::Token { cmd: TokenSubcommand::Create(args) } => run_token_create(args),
    }
}

// ── serve ────────────────────────────────────────────────────────────────────

async fn run_serve(args: crate::cli::ServeArgs) -> anyhow::Result<()> {
    let config = Config::resolve(&args)?;

    init_tracing(&config.log_level);

    // Step 3: データディレクトリ初期化
    DataDir::init(&config.data_dir)?;

    // Step 4: プロセス排他ロック
    let _lock = ProcessLock::acquire(&config.data_dir)?;

    // Step 6: DB 全件オープン
    let db_mgr = Arc::new(
        DbManager::open_all(&config.data_dir, Arc::new(config.storage.clone())).await?
    );

    tracing::info!(
        port       = config.port,
        admin_port = config.admin_port,
        "Adlaire DB Phase 2 ready (HTTP server not yet implemented)"
    );

    // Phase 3 でここに axum サーバー起動を追加する
    drop(db_mgr);

    anyhow::bail!("HTTP server not yet implemented (Phase 3 stub)")
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

#[allow(dead_code)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint  = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigint.recv()  => {},
        _ = sigterm.recv() => {},
    }
}
