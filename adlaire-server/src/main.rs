mod auth;
mod cli;
mod config;

use clap::Parser;

use crate::{
    auth::AccessLevel,
    cli::{Cli, CliCommand, TokenSubcommand},
    config::Config,
};

// ── エントリポイント ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommand::Serve(args)                         => run_serve(args).await,
        CliCommand::Token { cmd: TokenSubcommand::Create(args) } => run_token_create(args),
    }
}

// ── serve ────────────────────────────────────────────────────────────────────

async fn run_serve(args: crate::cli::ServeArgs) -> anyhow::Result<()> {
    let config = Config::resolve(&args)?;

    init_tracing(&config.log_level);

    tracing::info!(
        data_dir    = %config.data_dir.display(),
        port        = config.port,
        admin_port  = config.admin_port,
        "Adlaire DB starting (Phase 1 stub)"
    );

    // Phase 2 以降で DataDir::init / DbManager::open_all / Router 構築を追加
    anyhow::bail!("serve is not yet implemented (Phase 1 stub)")
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
    println!("(Phase 1 stub) token create — secret len={}", secret.len());
    Ok(())
}

// ── tracing 初期化 ────────────────────────────────────────────────────────────

fn init_tracing(log_level: &str) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

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
