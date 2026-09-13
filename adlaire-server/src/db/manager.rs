use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::RwLock;

use super::{
    meta::{
        DatabasesMeta, GroupInfo, LocationInfo, ManagementMeta, OrganizationInfo, QuotaInfo,
        UsageInfo,
    },
    sqld_adapter::{RealSqldAdapter, SqldAdapter},
    DbInfo,
};
use crate::{
    config::StorageConfig,
    db::{validate_db_name, validate_turso_db_name},
    error::AppError,
};

pub struct DbManager {
    data_dir: PathBuf,
    dbs:      RwLock<HashMap<String, Arc<dyn SqldAdapter>>>,
    meta:     RwLock<DatabasesMeta>,
    management: RwLock<ManagementMeta>,
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
                id:                uuid::Uuid::new_v4().to_string(),
                name:              "default".to_string(),
                created_at:        chrono::Utc::now(),
                size_bytes:        0,
                organization:      "default".to_string(),
                group:             "default".to_string(),
                location:          "default".to_string(),
                delete_protection: false,
                block_reads:       false,
                block_writes:      false,
                allow_attach:      true,
                legacy_name:       false,
            };
            let db_path = data_dir.join("databases").join("default");
            std::fs::create_dir_all(&db_path)?;
            meta.databases.push(info);
            meta.save(data_dir)?;
        }

        let mut changed = false;
        for db in &mut meta.databases {
            if db.organization.is_empty() {
                db.organization = "default".to_string();
                changed = true;
            }
            if db.group.is_empty() {
                db.group = "default".to_string();
                changed = true;
            }
            if db.location.is_empty() {
                db.location = "default".to_string();
                changed = true;
            }
            if db.name.contains('_') || db.name.chars().any(|c| c.is_ascii_uppercase()) || db.name.len() > 64 {
                db.legacy_name = true;
                changed = true;
            }
        }
        if changed {
            meta.save(data_dir)?;
        }
        let management = ManagementMeta::load_or_init(data_dir)?;

        let mut dbs: HashMap<String, Arc<dyn SqldAdapter>> = HashMap::new();
        for db in &meta.databases {
            let db_file = data_dir
                .join("databases")
                .join(&db.name)
                .join("data.db");

            match RealSqldAdapter::open(&db_file, config.busy_timeout_ms, !config.skip_integrity_check).await {
                Ok(adapter) => {
                    dbs.insert(db.name.clone(), Arc::new(adapter));
                    tracing::info!(db = %db.name, "opened database");
                }
                Err(e) => {
                    tracing::error!(db = %db.name, err = %e, "failed to open database");
                    anyhow::bail!("failed to open database '{}': {e}", db.name);
                }
            }
        }

        tracing::info!(count = dbs.len(), "DbManager ready");
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            dbs:      RwLock::new(dbs),
            meta:     RwLock::new(meta),
            management: RwLock::new(management),
            config,
        })
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn SqldAdapter>> {
        self.dbs.read().await.get(name).cloned()
    }

    pub async fn create(&self, name: &str) -> Result<DbInfo, AppError> {
        self.create_inner(name, "default", "default", "default", false).await
    }

    pub async fn create_scoped(
        &self,
        name: &str,
        organization: &str,
        group: &str,
        location: &str,
    ) -> Result<DbInfo, AppError> {
        let organization = self.resolve_organization_slug(organization).await?;
        let group = self.resolve_group_name(&organization, group).await?;
        let location = self.resolve_location_name(location).await?;
        self.create_inner(name, &organization, &group, &location, true).await
    }

    async fn create_inner(
        &self,
        name: &str,
        organization: &str,
        group: &str,
        location: &str,
        turso_name: bool,
    ) -> Result<DbInfo, AppError> {
        if turso_name {
            validate_turso_db_name(name)?;
        } else {
            validate_db_name(name)?;
        }
        self.require_organization(organization).await?;
        self.require_group(organization, group).await?;
        self.require_location(location).await?;

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
        let adapter = RealSqldAdapter::open(&db_file, self.config.busy_timeout_ms, !self.config.skip_integrity_check)
            .await
            .map_err(|e| AppError::Internal(e))?;

        let info = DbInfo {
            id:                uuid::Uuid::new_v4().to_string(),
            name:              name.to_string(),
            created_at:        chrono::Utc::now(),
            size_bytes:        0,
            organization:      organization.to_string(),
            group:             group.to_string(),
            location:          location.to_string(),
            delete_protection: false,
            block_reads:       false,
            block_writes:      false,
            allow_attach:      true,
            legacy_name:       false,
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
            let Some(info) = meta.databases.iter().find(|d| d.name == name) else {
                return Err(AppError::DbNotFound(name.to_string()));
            };
            if info.delete_protection {
                return Err(AppError::OrgScopeDenied);
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

    pub async fn list_for_organization(&self, organization: &str, group: Option<&str>) -> Result<Vec<DbInfo>, AppError> {
        let organization = self.resolve_organization_slug(organization).await?;
        let group = match group {
            Some(group) => Some(self.resolve_group_name(&organization, group).await?),
            None => None,
        };
        Ok(self
            .list()
            .await
            .into_iter()
            .filter(|db| db.organization == organization)
            .filter(|db| group.as_ref().map(|group| db.group == *group).unwrap_or(true))
            .collect())
    }

    pub async fn organizations(&self) -> Vec<OrganizationInfo> {
        self.management.read().await.organizations.clone()
    }

    pub async fn create_organization(&self, name: String, slug: Option<String>) -> Result<OrganizationInfo, AppError> {
        let slug = slug.unwrap_or_else(|| name.clone());
        validate_slug(&name)?;
        validate_slug(&slug)?;
        let mut management = self.management.write().await;
        if management.organizations.iter().any(|org| org.id == slug || org.slug == slug) {
            return Err(AppError::OrgAlreadyExists(slug));
        }
        let info = OrganizationInfo {
            id: format!("org_{}", uuid::Uuid::new_v4().simple()),
            name,
            slug,
            created_at: chrono::Utc::now(),
        };
        management.organizations.push(info.clone());
        management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    pub async fn groups(&self, organization: Option<&str>) -> Result<Vec<GroupInfo>, AppError> {
        let organization = match organization {
            Some(org) => Some(self.resolve_organization_slug(org).await?),
            None => None,
        };
        let groups = self.management.read().await.groups
            .iter()
            .filter(|group| organization.as_ref().map(|org| group.organization == *org).unwrap_or(true))
            .cloned()
            .collect();
        Ok(groups)
    }

    pub async fn create_group(
        &self,
        organization: String,
        name: String,
        slug: Option<String>,
        location: Option<String>,
    ) -> Result<GroupInfo, AppError> {
        let slug = slug.unwrap_or_else(|| name.clone());
        let location = location.unwrap_or_else(|| "default".to_string());
        validate_slug(&organization)?;
        validate_slug(&name)?;
        validate_slug(&slug)?;
        let organization = self.resolve_organization_slug(&organization).await?;
        let location = self.resolve_location_name(&location).await?;
        let mut management = self.management.write().await;
        if management.groups.iter().any(|group| {
            group.organization == organization && (group.name == name || group.slug == slug)
        }) {
            return Err(AppError::GroupAlreadyExists(slug));
        }
        let info = GroupInfo {
            id: format!("grp_{}", uuid::Uuid::new_v4().simple()),
            organization,
            name,
            slug,
            location,
            delete_protection: false,
            created_at: chrono::Utc::now(),
        };
        management.groups.push(info.clone());
        management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    pub async fn locations(&self) -> Vec<LocationInfo> {
        self.management.read().await.locations.clone()
    }

    pub async fn create_location(
        &self,
        name: String,
        provider: Option<String>,
        region: Option<String>,
        primary: Option<bool>,
    ) -> Result<LocationInfo, AppError> {
        validate_slug(&name)?;
        let mut management = self.management.write().await;
        if management.locations.iter().any(|loc| loc.name == name || loc.id == name) {
            return Err(AppError::LocationAlreadyExists(name));
        }
        let info = LocationInfo {
            id: format!("loc_{}", uuid::Uuid::new_v4().simple()),
            name,
            provider: provider.unwrap_or_else(|| "self-hosted".to_string()),
            region: region.unwrap_or_else(|| "local".to_string()),
            primary: primary.unwrap_or(false),
            created_at: chrono::Utc::now(),
        };
        management.locations.push(info.clone());
        management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    pub async fn quotas(&self) -> Vec<QuotaInfo> {
        self.management.read().await.quotas.clone()
    }

    pub async fn put_quota(
        &self,
        scope_type: String,
        scope: String,
        storage_bytes: Option<u64>,
        rows: Option<u64>,
        write_ops_per_minute: Option<u64>,
    ) -> Result<QuotaInfo, AppError> {
        self.require_scope(&scope_type, &scope).await?;
        let mut management = self.management.write().await;
        let now = chrono::Utc::now();
        if let Some(existing) = management.quotas.iter_mut().find(|quota| quota.scope_type == scope_type && quota.scope == scope) {
            existing.storage_bytes = storage_bytes;
            existing.rows = rows;
            existing.write_ops_per_minute = write_ops_per_minute;
            existing.updated_at = now;
            let info = existing.clone();
            management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
            return Ok(info);
        }
        let info = QuotaInfo { scope_type, scope, storage_bytes, rows, write_ops_per_minute, updated_at: now };
        management.quotas.push(info.clone());
        management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    pub async fn usage(&self) -> Vec<UsageInfo> {
        let mut usage = self.management.read().await.usage.clone();
        for item in &mut usage {
            if item.scope_type == "database" {
                if let Ok(info) = self.get_info(&item.scope).await {
                    item.storage_bytes = info.size_bytes;
                }
            }
        }
        usage
    }

    pub async fn quota_exceeded_for_db(&self, name: &str) -> Result<bool, AppError> {
        let info = self.get_info(name).await?;
        let quotas = self.management.read().await.quotas.clone();
        for quota in quotas {
            let applies = match quota.scope_type.as_str() {
                "organization" => quota.scope == info.organization,
                "group" => quota.scope == info.group,
                "database" => quota.scope == info.name,
                _ => false,
            };
            if applies {
                if let Some(limit) = quota.storage_bytes {
                    if info.size_bytes >= limit {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
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

    pub async fn update_db_configuration(
        &self,
        name: &str,
        delete_protection: Option<bool>,
        block_reads: Option<bool>,
        block_writes: Option<bool>,
        allow_attach: Option<bool>,
    ) -> Result<DbInfo, AppError> {
        let mut meta = self.meta.write().await;
        let Some(info) = meta.databases.iter_mut().find(|db| db.name == name) else {
            return Err(AppError::DbNotFound(name.to_string()));
        };
        if let Some(value) = delete_protection {
            info.delete_protection = value;
        }
        if let Some(value) = block_reads {
            info.block_reads = value;
        }
        if let Some(value) = block_writes {
            info.block_writes = value;
        }
        if let Some(value) = allow_attach {
            info.allow_attach = value;
        }
        let info = info.clone();
        meta.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    pub async fn group_info(&self, organization: &str, group: &str) -> Result<GroupInfo, AppError> {
        let organization = self.resolve_organization_slug(organization).await?;
        let management = self.management.read().await;
        management
            .groups
            .iter()
            .find(|item| {
                item.organization == organization
                    && (item.id == group || item.slug == group || item.name == group)
            })
            .cloned()
            .ok_or_else(|| AppError::GroupNotFound(group.to_string()))
    }

    pub async fn update_group_configuration(
        &self,
        organization: &str,
        group: &str,
        delete_protection: bool,
    ) -> Result<GroupInfo, AppError> {
        let organization = self.resolve_organization_slug(organization).await?;
        let mut management = self.management.write().await;
        let Some(info) = management.groups.iter_mut().find(|item| {
            item.organization == organization
                && (item.id == group || item.slug == group || item.name == group)
        }) else {
            return Err(AppError::GroupNotFound(group.to_string()));
        };
        info.delete_protection = delete_protection;
        let info = info.clone();
        management.save(&self.data_dir).map_err(|e| AppError::Internal(e))?;
        Ok(info)
    }

    async fn require_organization(&self, org: &str) -> Result<(), AppError> {
        if self.management.read().await.organizations.iter().any(|item| item.id == org || item.slug == org || item.name == org) {
            Ok(())
        } else {
            Err(AppError::OrgNotFound(org.to_string()))
        }
    }

    async fn require_group(&self, organization: &str, group: &str) -> Result<(), AppError> {
        if self.management.read().await.groups.iter().any(|item| {
            item.organization == organization && (item.id == group || item.slug == group || item.name == group)
        }) {
            Ok(())
        } else {
            Err(AppError::GroupNotFound(group.to_string()))
        }
    }

    async fn require_location(&self, location: &str) -> Result<(), AppError> {
        if self.management.read().await.locations.iter().any(|item| item.id == location || item.name == location) {
            Ok(())
        } else {
            Err(AppError::LocationNotFound(location.to_string()))
        }
    }

    async fn require_scope(&self, scope_type: &str, scope: &str) -> Result<(), AppError> {
        match scope_type {
            "organization" => self.require_organization(scope).await,
            "group" => {
                if self.management.read().await.groups.iter().any(|item| item.id == scope || item.slug == scope || item.name == scope) {
                    Ok(())
                } else {
                    Err(AppError::GroupNotFound(scope.to_string()))
                }
            }
            "database" => self.get_info(scope).await.map(|_| ()),
            _ => Err(AppError::InvalidRequest),
        }
    }

    async fn resolve_organization_slug(&self, org: &str) -> Result<String, AppError> {
        self.management
            .read()
            .await
            .organizations
            .iter()
            .find(|item| item.id == org || item.slug == org || item.name == org)
            .map(|item| item.slug.clone())
            .ok_or_else(|| AppError::OrgNotFound(org.to_string()))
    }

    async fn resolve_group_name(&self, organization: &str, group: &str) -> Result<String, AppError> {
        self.management
            .read()
            .await
            .groups
            .iter()
            .find(|item| {
                item.organization == organization
                    && (item.id == group || item.slug == group || item.name == group)
            })
            .map(|item| item.name.clone())
            .ok_or_else(|| AppError::GroupNotFound(group.to_string()))
    }

    async fn resolve_location_name(&self, location: &str) -> Result<String, AppError> {
        self.management
            .read()
            .await
            .locations
            .iter()
            .find(|item| item.id == location || item.name == location)
            .map(|item| item.name.clone())
            .ok_or_else(|| AppError::LocationNotFound(location.to_string()))
    }

    pub async fn close_all(self) {
        let count = self.dbs.read().await.len();
        tracing::info!(count, "closing all databases");
    }
}

fn validate_slug(value: &str) -> Result<(), AppError> {
    use std::sync::LazyLock;
    use regex::Regex;
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_-]{1,63}$").unwrap());
    if RE.is_match(value) {
        Ok(())
    } else {
        Err(AppError::InvalidRequest)
    }
}
