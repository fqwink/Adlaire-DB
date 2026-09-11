use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use tokio::sync::RwLock;

use crate::{config::StorageConfig, error::AppError};
use super::{
    meta::DatabasesMeta,
    sqld_adapter::{RealSqldAdapter, SqldAdapter},
    DbInfo,
};

pub struct DbManager {
    data_dir: PathBuf,
    dbs:      RwLock<HashMap<String, Arc<dyn SqldAdapter>>>,
    meta:     RwLock<DatabasesMeta>,
    config:   Arc<StorageConfig>,
}

impl DbManager {
    /// 起動時: databases.json を読み込み、各 DB をオープンする
    pub async fn open_all(
        data_dir: &Path,
        config:   Arc<StorageConfig>,
    ) -> anyhow::Result<Self> {
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
            let db_file = data_dir
                .join("databases")
                .join(&db.name)
                .join("data.db");

            match RealSqldAdapter::open(&db_file, config.busy_timeout_ms).await {
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
            dbs:      RwLock::new(dbs),
            meta:     RwLock::new(meta),
            config,
        })
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn SqldAdapter>> {
        self.dbs.read().await.get(name).cloned()
    }

    pub async fn create(&self, name: &str) -> Result<DbInfo, AppError> {
        {
            let meta = self.meta.read().await;
            if meta.databases.iter().any(|d| d.name == name) {
                return Err(AppError::DbAlreadyExists(name.to_string()));
            }
        }

        let db_path = self.data_dir.join("databases").join(name);
        std::fs::create_dir_all(&db_path)
            .map_err(|e| AppError::Internal(e.into()))?;

        let db_file = db_path.join("data.db");
        let adapter = RealSqldAdapter::open(&db_file, self.config.busy_timeout_ms)
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
            std::fs::remove_dir_all(&db_path)
                .map_err(|e| AppError::Internal(e.into()))?;
        }

        {
            let mut meta = self.meta.write().await;
            meta.databases.retain(|d| d.name != name);
            meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        }

        tracing::info!(db = name, "deleted database");
        Ok(())
    }

    pub async fn list(&self) -> Vec<DbInfo> {
        let meta = self.meta.read().await;
        meta.databases
            .iter()
            .map(|info| {
                let db_path = self.data_dir
                    .join("databases")
                    .join(&info.name)
                    .join("data.db");
                let size_bytes = std::fs::metadata(&db_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                DbInfo { size_bytes, ..info.clone() }
            })
            .collect()
    }

    pub async fn get_info(&self, name: &str) -> Result<DbInfo, AppError> {
        let meta = self.meta.read().await;
        let info = meta
            .databases
            .iter()
            .find(|d| d.name == name)
            .cloned()
            .ok_or_else(|| AppError::DbNotFound(name.to_string()))?;
        let db_path = self.data_dir
            .join("databases")
            .join(name)
            .join("data.db");
        let size_bytes = std::fs::metadata(&db_path)
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(DbInfo { size_bytes, ..info })
    }

    pub async fn close_all(self) {
        let count = self.dbs.read().await.len();
        tracing::info!(count, "closing all databases");
    }
}
