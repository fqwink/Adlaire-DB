use std::{os::unix::fs::PermissionsExt, path::Path};

pub struct DataDir;

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
        // §8.1 Step 5-2: tokens.json が存在しなければ空で初期化する
        let tokens_path = data_dir.join("meta").join("tokens.json");
        if !tokens_path.exists() {
            std::fs::write(&tokens_path, r#"{"tokens":[]}"#)?;
        }

        tracing::info!(path = %data_dir.display(), "DataDir initialized");
        Ok(())
    }
}

// ── プロセス多重起動防止 ──────────────────────────────────────────────────────

pub struct ProcessLock {
    _file: std::fs::File, // Drop 時に flock が自動解放される
}

impl ProcessLock {
    pub fn acquire(data_dir: &Path) -> anyhow::Result<Self> {
        use std::os::unix::io::AsRawFd;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(data_dir.join(".lock"))?;
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            anyhow::bail!(
                "another adlaire-db process is already running in {:?}",
                data_dir
            );
        }
        Ok(Self { _file: file })
    }
}
