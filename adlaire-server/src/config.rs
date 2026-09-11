use std::{path::PathBuf, sync::Arc};

use crate::cli::ServeArgs;

// ── 公開型 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir:             PathBuf,
    pub port:                 u16,
    pub admin_port:           u16,
    pub log_level:            String,
    pub admin_auth_token:     Option<String>,
    pub jwt_secret_bytes:     Option<Vec<u8>>,
    pub shutdown_timeout:     u64,
    pub skip_integrity_check: bool,
    pub storage:              StorageConfig,
    pub replication:          ReplicationConfig,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub busy_timeout_ms:              u64,
    pub wal_checkpoint_pages:         u64,
    pub wal_checkpoint_mode:          WalCheckpointMode,
    pub wal_retention_days:           u64,
    pub integrity_check_interval_hrs: u64,
    pub skip_integrity_check:         bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalCheckpointMode {
    Passive,
    Full,
    Restart,
}

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub write_mode:      ReplicationWriteMode,
    pub sync_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationWriteMode {
    Async,
    Sync,
}

// ── TOML 構造体 ───────────────────────────────────────────────────────────────

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
    pub auth_token: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlStorage {
    pub wal_mode:             Option<String>,
    pub skip_integrity_check: Option<bool>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TomlReplication {
    pub write_mode: Option<String>,
}

// ── Config::resolve ───────────────────────────────────────────────────────────

impl Config {
    pub fn tokens_path(&self) -> PathBuf {
        self.data_dir.join("meta").join("tokens.json")
    }

    pub fn resolve(args: &ServeArgs) -> anyhow::Result<Arc<Config>> {
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
        let adm  = toml.admin.unwrap_or_default();
        let sto  = toml.storage.unwrap_or_default();
        let rep  = toml.replication.unwrap_or_default();

        // JWT シークレット解決（CLI > env > TOML > TOML file）
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

        if let Some(ref b) = raw_secret {
            anyhow::ensure!(b.len() >= 32, "JWT secret must be at least 32 bytes");
        }

        let port       = args.port.or(srv.port).unwrap_or(8080);
        let admin_port = args.admin_port.or(srv.admin_port).unwrap_or(8081);
        let log_level  = args.log_level.as_deref()
            .or(srv.log_level.as_deref())
            .unwrap_or("info")
            .to_string();
        let busy_timeout_ms   = args.busy_timeout.or(srv.busy_timeout_ms).unwrap_or(5000);
        let shutdown_timeout  = args.shutdown_timeout.or(srv.shutdown_timeout).unwrap_or(30);
        let skip_integrity_check = args.skip_integrity_check
            || sto.skip_integrity_check.unwrap_or(false);
        let wal_mode   = parse_wal_mode(sto.wal_mode.as_deref())?;
        let write_mode = match &args.replication_write_mode {
            Some(s) => parse_write_mode(s)?,
            None    => rep.write_mode.as_deref()
                          .map(parse_write_mode)
                          .transpose()?
                          .unwrap_or(ReplicationWriteMode::Async),
        };
        let admin_auth_token = args.admin_auth_token.clone()
            .or(adm.auth_token)
            .or_else(|| std::env::var("ADLAIRE_ADMIN_TOKEN").ok());

        Ok(Arc::new(Config {
            data_dir: args.data.clone(),
            port,
            admin_port,
            log_level,
            admin_auth_token,
            jwt_secret_bytes: raw_secret,
            shutdown_timeout,
            skip_integrity_check,
            storage: StorageConfig {
                busy_timeout_ms,
                wal_checkpoint_pages:         1000,
                wal_checkpoint_mode:          wal_mode,
                wal_retention_days:           0,
                integrity_check_interval_hrs: 0,
                skip_integrity_check,
            },
            replication: ReplicationConfig {
                write_mode,
                sync_timeout_ms: 5000,
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
