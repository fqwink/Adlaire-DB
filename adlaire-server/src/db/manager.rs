use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use tokio::sync::RwLock;

use crate::{config::StorageConfig, error::AppError};
use super::{
    meta::DatabasesMeta,
    sqld_adapter::{MockSqldAdapter, SqldAdapter},
    DbInfo,
};

pub struct DbManager {
    data_dir: PathBuf,
    dbs:      RwLock<HashMap<String, Arc<dyn SqldAdapter>>>,
    meta:     RwLock<DatabasesMeta>,
    #[allow(dead_code)]
    config:   Arc<StorageConfig>,
}

impl DbManager {
    /// 起動時: databases.json を読み込み、各 DB ディレクトリを確認してオープン
    pub async fn open_all(
        data_dir: &Path,
        config:   Arc<StorageConfig>,
    ) -> anyhow::Result<Self> {
        let meta = DatabasesMeta::load(data_dir)?;

        let mut dbs: HashMap<String, Arc<dyn SqldAdapter>> = HashMap::new();
        for db in &meta.databases {
            let db_path = data_dir.join("databases").join(&db.name);
            if db_path.exists() {
                // Phase 3 で RealSqldAdapter に差し替える
                dbs.insert(db.name.clone(), Arc::new(MockSqldAdapter));
                tracing::info!(db = %db.name, "opened database");
            } else {
                tracing::warn!(db = %db.name, "database directory missing, skipping");
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

    /// DB 名 → SqldAdapter を返す
    pub async fn get(&self, name: &str) -> Option<Arc<dyn SqldAdapter>> {
        self.dbs.read().await.get(name).cloned()
    }

    /// DB 作成: ディレクトリ作成 → adapter オープン → meta 更新
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

        self.dbs
            .write()
            .await
            .insert(name.to_string(), Arc::new(MockSqldAdapter));

        tracing::info!(db = name, id = %info.id, "created database");
        Ok(info)
    }

    /// DB 削除: adapter 除去 → ディレクトリ削除 → meta 更新
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

    /// DB 一覧（size_bytes は data.db のファイルサイズを動的取得）
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

    /// DB 名で DbInfo を返す（存在しない場合は DbNotFound）
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

    /// シャットダウン時: 全 adapter を drop（WAL flush は Phase 3 で sqld 統合後に実装）
    pub async fn close_all(self) {
        let count = self.dbs.read().await.len();
        tracing::info!(count, "closing all databases");
        // dbs の drop で各 Arc<dyn SqldAdapter> が解放される
    }
}
