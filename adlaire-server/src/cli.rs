use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "adlaire-db", version, about = "Self-hosted libSQL-compatible DB server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand)]
pub enum CliCommand {
    Serve(ServeArgs),
    Token {
        #[command(subcommand)]
        cmd: TokenSubcommand,
    },
}

#[derive(Subcommand)]
pub enum TokenSubcommand {
    Create(TokenCreateArgs),
}

#[derive(Parser)]
pub struct ServeArgs {
    #[arg(long, required = true)]
    pub data: PathBuf,

    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long)]
    pub admin_port: Option<u16>,

    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(long)]
    pub auth_jwt_secret: Option<String>,

    #[arg(long)]
    pub auth_jwt_secret_file: Option<PathBuf>,

    #[arg(long)]
    pub admin_auth_token: Option<String>,

    #[arg(long)]
    pub log_level: Option<String>,

    #[arg(long, default_value_t = false)]
    pub skip_integrity_check: bool,

    #[arg(long)]
    pub replication_write_mode: Option<String>,

    #[arg(long)]
    pub busy_timeout: Option<u64>,

    #[arg(long)]
    pub shutdown_timeout: Option<u64>,
}

#[derive(Parser)]
pub struct TokenCreateArgs {
    #[arg(long, required = true)]
    pub secret: String,

    #[arg(long, default_value = "rw")]
    pub access: String,

    #[arg(long)]
    pub expiry: Option<String>,

    #[arg(long, value_name = "DB:ACCESS")]
    pub db: Vec<String>,
}
