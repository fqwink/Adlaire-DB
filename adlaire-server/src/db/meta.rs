use std::{collections::HashSet, fs::OpenOptions, io::Write, path::Path};

use super::DbInfo;

const PHASE8_MARKER: &str = "phase8-migration.json";

fn default_id(prefix: &str) -> String {
    format!("{prefix}_default")
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabasesMeta {
    pub databases: Vec<DbInfo>,
}

impl DatabasesMeta {
    pub fn load(data_dir: &Path) -> anyhow::Result<Self> {
        let path = data_dir.join("meta").join("databases.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        validate_database_constraints(self)?;
        let path = data_dir.join("meta").join("databases.json");
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        atomic_write_json(data_dir, &tmp, &path, &json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrganizationInfo {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub organization: String,
    pub name: String,
    pub slug: String,
    pub location: String,
    pub delete_protection: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocationInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub region: String,
    pub primary: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuotaInfo {
    pub scope_type: String,
    pub scope: String,
    pub storage_bytes: Option<u64>,
    pub rows: Option<u64>,
    pub write_ops_per_minute: Option<u64>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageInfo {
    pub scope_type: String,
    pub scope: String,
    pub storage_bytes: u64,
    pub rows: Option<u64>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ManagementMeta {
    pub organizations: Vec<OrganizationInfo>,
    pub groups: Vec<GroupInfo>,
    pub locations: Vec<LocationInfo>,
    pub quotas: Vec<QuotaInfo>,
    pub usage: Vec<UsageInfo>,
}

impl ManagementMeta {
    pub fn load_or_init(data_dir: &Path) -> anyhow::Result<Self> {
        preflight_phase8(data_dir)?;
        let backup = ensure_phase8_backup(data_dir)?;
        let now = chrono::Utc::now();
        let mut organizations: Vec<OrganizationInfo> =
            load_list(data_dir, "organizations.json", "organizations")?;
        let mut groups: Vec<GroupInfo> = load_list(data_dir, "groups.json", "groups")?;
        let mut locations: Vec<LocationInfo> = load_list(data_dir, "locations.json", "locations")?;
        let mut quotas: Vec<QuotaInfo> = load_list(data_dir, "quotas.json", "quotas")?;
        let mut usage: Vec<UsageInfo> = load_list(data_dir, "usage.json", "usage")?;

        if organizations.is_empty() {
            organizations.push(OrganizationInfo {
                id: default_id("org"),
                name: "default".to_string(),
                slug: "default".to_string(),
                created_at: now,
            });
        }
        if locations.is_empty() {
            locations.push(LocationInfo {
                id: default_id("loc"),
                name: "default".to_string(),
                provider: "self-hosted".to_string(),
                region: "local".to_string(),
                primary: true,
                created_at: now,
            });
        }
        if groups.is_empty() {
            groups.push(GroupInfo {
                id: default_id("grp"),
                organization: "default".to_string(),
                name: "default".to_string(),
                slug: "default".to_string(),
                location: "default".to_string(),
                delete_protection: false,
                created_at: now,
            });
        }
        if quotas.is_empty() {
            quotas.push(QuotaInfo {
                scope_type: "organization".to_string(),
                scope: "default".to_string(),
                storage_bytes: Some(10 * 1024 * 1024 * 1024),
                rows: None,
                write_ops_per_minute: None,
                updated_at: now,
            });
        }
        if usage.is_empty() {
            usage.push(UsageInfo {
                scope_type: "organization".to_string(),
                scope: "default".to_string(),
                storage_bytes: 0,
                rows: None,
                updated_at: now,
            });
        }

        let meta = Self {
            organizations,
            groups,
            locations,
            quotas,
            usage,
        };
        validate_management_constraints(&meta)?;
        meta.save(data_dir)?;
        crate::token::util::ensure_phase8_token_metadata(data_dir)?;
        write_phase8_marker(data_dir, backup.as_deref())?;
        Ok(meta)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        validate_management_constraints(self)?;
        save_list(
            data_dir,
            "organizations.json",
            "organizations",
            &self.organizations,
        )?;
        save_list(data_dir, "groups.json", "groups", &self.groups)?;
        save_list(data_dir, "locations.json", "locations", &self.locations)?;
        save_list(data_dir, "quotas.json", "quotas", &self.quotas)?;
        save_list(data_dir, "usage.json", "usage", &self.usage)?;
        Ok(())
    }
}

pub fn validate_database_constraints(meta: &DatabasesMeta) -> anyhow::Result<()> {
    let mut names = HashSet::new();
    let mut org_names = HashSet::new();
    for db in &meta.databases {
        anyhow::ensure!(!db.id.is_empty(), "database id is required");
        anyhow::ensure!(!db.name.is_empty(), "database name is required");
        anyhow::ensure!(
            names.insert(db.name.clone()),
            "duplicate database name: {}",
            db.name
        );
        anyhow::ensure!(
            org_names.insert((db.organization.clone(), db.name.clone())),
            "duplicate database in organization: {}/{}",
            db.organization,
            db.name
        );
    }
    Ok(())
}

pub fn validate_management_constraints(meta: &ManagementMeta) -> anyhow::Result<()> {
    let mut org_ids = HashSet::new();
    let mut org_slugs = HashSet::new();
    for org in &meta.organizations {
        anyhow::ensure!(!org.id.is_empty(), "organization id is required");
        anyhow::ensure!(!org.slug.is_empty(), "organization slug is required");
        anyhow::ensure!(
            org_ids.insert(org.id.clone()),
            "duplicate organization id: {}",
            org.id
        );
        anyhow::ensure!(
            org_slugs.insert(org.slug.clone()),
            "duplicate organization slug: {}",
            org.slug
        );
    }

    let mut group_ids = HashSet::new();
    let mut group_names = HashSet::new();
    let mut group_slugs = HashSet::new();
    for group in &meta.groups {
        anyhow::ensure!(
            !group.organization.is_empty(),
            "group organization is required"
        );
        anyhow::ensure!(!group.id.is_empty(), "group id is required");
        anyhow::ensure!(!group.name.is_empty(), "group name is required");
        anyhow::ensure!(!group.slug.is_empty(), "group slug is required");
        anyhow::ensure!(
            group_ids.insert((group.organization.clone(), group.id.clone())),
            "duplicate group id: {}/{}",
            group.organization,
            group.id
        );
        anyhow::ensure!(
            group_names.insert((group.organization.clone(), group.name.clone())),
            "duplicate group name: {}/{}",
            group.organization,
            group.name
        );
        anyhow::ensure!(
            group_slugs.insert((group.organization.clone(), group.slug.clone())),
            "duplicate group slug: {}/{}",
            group.organization,
            group.slug
        );
    }

    let mut location_ids = HashSet::new();
    let mut location_names = HashSet::new();
    for location in &meta.locations {
        anyhow::ensure!(!location.id.is_empty(), "location id is required");
        anyhow::ensure!(!location.name.is_empty(), "location name is required");
        anyhow::ensure!(
            location_ids.insert(location.id.clone()),
            "duplicate location id: {}",
            location.id
        );
        anyhow::ensure!(
            location_names.insert(location.name.clone()),
            "duplicate location name: {}",
            location.name
        );
    }

    let mut quota_scopes = HashSet::new();
    for quota in &meta.quotas {
        anyhow::ensure!(!quota.scope_type.is_empty(), "quota scope_type is required");
        anyhow::ensure!(!quota.scope.is_empty(), "quota scope is required");
        anyhow::ensure!(
            quota_scopes.insert((quota.scope_type.clone(), quota.scope.clone())),
            "duplicate quota scope: {}:{}",
            quota.scope_type,
            quota.scope
        );
    }

    let mut usage_scopes = HashSet::new();
    for usage in &meta.usage {
        anyhow::ensure!(!usage.scope_type.is_empty(), "usage scope_type is required");
        anyhow::ensure!(!usage.scope.is_empty(), "usage scope is required");
        anyhow::ensure!(
            usage_scopes.insert((usage.scope_type.clone(), usage.scope.clone())),
            "duplicate usage scope: {}:{}",
            usage.scope_type,
            usage.scope
        );
    }
    Ok(())
}

fn preflight_phase8(data_dir: &Path) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    let databases = DatabasesMeta::load(data_dir)?;
    validate_database_constraints(&databases)?;
    for db in &databases.databases {
        let db_dir = data_dir.join("databases").join(&db.name);
        anyhow::ensure!(db_dir.exists(), "database directory missing: {}", db.name);
    }
    let _ = crate::token::util::load_tokens(data_dir)?;
    std::fs::create_dir_all(meta_dir)?;
    Ok(())
}

fn ensure_phase8_backup(data_dir: &Path) -> anyhow::Result<Option<String>> {
    let meta_dir = data_dir.join("meta");
    if meta_dir.join(PHASE8_MARKER).exists() {
        return Ok(None);
    }
    rollback_incomplete_phase8(data_dir)?;
    let backup_name = format!("phase8-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let backup_dir = meta_dir.join("migration-backup").join(&backup_name);
    std::fs::create_dir_all(&backup_dir)?;
    for file_name in [
        "databases.json",
        "tokens.json",
        "organizations.json",
        "groups.json",
        "locations.json",
        "quotas.json",
        "usage.json",
    ] {
        let source = meta_dir.join(file_name);
        if source.exists() {
            let target = backup_dir.join(file_name);
            std::fs::copy(source, target)?;
        }
    }
    sync_dir(&backup_dir)?;
    sync_dir(&meta_dir)?;
    Ok(Some(format!("migration-backup/{backup_name}")))
}

fn rollback_incomplete_phase8(data_dir: &Path) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    let backup_root = meta_dir.join("migration-backup");
    if !backup_root.exists() {
        return Ok(());
    }
    let mut backups = std::fs::read_dir(&backup_root)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("phase8-"))
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| entry.file_name());
    let Some(backup) = backups.pop() else {
        return Ok(());
    };
    for file_name in [
        "databases.json",
        "tokens.json",
        "organizations.json",
        "groups.json",
        "locations.json",
        "quotas.json",
        "usage.json",
    ] {
        let source = backup.path().join(file_name);
        if source.exists() {
            let target = meta_dir.join(file_name);
            std::fs::copy(source, target)?;
        }
    }
    sync_dir(&meta_dir)?;
    anyhow::bail!(
        "incomplete Phase 8 migration restored from {}; restart required",
        backup.path().display()
    );
}

fn write_phase8_marker(data_dir: &Path, backup: Option<&str>) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    let path = meta_dir.join(PHASE8_MARKER);
    if path.exists() {
        return Ok(());
    }
    let tmp = path.with_extension("json.tmp");
    let value = serde_json::json!({
        "from_phase": 7,
        "completed_at": chrono::Utc::now(),
        "backup": backup.unwrap_or(""),
    });
    let json = serde_json::to_vec_pretty(&value)?;
    atomic_write_json(data_dir, &tmp, &path, &json)?;
    Ok(())
}

fn load_list<T: serde::de::DeserializeOwned>(
    data_dir: &Path,
    file_name: &str,
    key: &str,
) -> anyhow::Result<Vec<T>> {
    let path = data_dir.join("meta").join(file_name);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let Some(items) = value.get(key) else {
        return Ok(Vec::new());
    };
    Ok(serde_json::from_value(items.clone())?)
}

fn save_list<T: serde::Serialize>(
    data_dir: &Path,
    file_name: &str,
    key: &str,
    items: &[T],
) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;
    let path = meta_dir.join(file_name);
    let tmp = path.with_extension("json.tmp");
    let value = serde_json::json!({ key: items });
    let json = serde_json::to_vec_pretty(&value)?;
    atomic_write_json(data_dir, &tmp, &path, &json)?;
    Ok(())
}

fn atomic_write_json(data_dir: &Path, tmp: &Path, path: &Path, json: &[u8]) -> anyhow::Result<()> {
    let meta_dir = data_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(tmp)?;
        file.write_all(json)?;
        file.sync_all()?;
    }
    std::fs::rename(tmp, path)?;
    sync_dir(&meta_dir)?;
    Ok(())
}

fn sync_dir(path: &Path) -> anyhow::Result<()> {
    let dir = OpenOptions::new().read(true).open(path)?;
    dir.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db(name: &str, organization: &str) -> DbInfo {
        DbInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
            organization: organization.to_string(),
            group: "default".to_string(),
            location: "default".to_string(),
            delete_protection: false,
            block_reads: false,
            block_writes: false,
            allow_attach: true,
            legacy_name: false,
        }
    }

    fn management() -> ManagementMeta {
        let now = chrono::Utc::now();
        ManagementMeta {
            organizations: vec![OrganizationInfo {
                id: "org_default".to_string(),
                name: "default".to_string(),
                slug: "default".to_string(),
                created_at: now,
            }],
            groups: vec![GroupInfo {
                id: "grp_default".to_string(),
                organization: "default".to_string(),
                name: "default".to_string(),
                slug: "default".to_string(),
                location: "default".to_string(),
                delete_protection: false,
                created_at: now,
            }],
            locations: vec![LocationInfo {
                id: "loc_default".to_string(),
                name: "default".to_string(),
                provider: "self-hosted".to_string(),
                region: "local".to_string(),
                primary: true,
                created_at: now,
            }],
            quotas: vec![QuotaInfo {
                scope_type: "organization".to_string(),
                scope: "default".to_string(),
                storage_bytes: Some(1),
                rows: None,
                write_ops_per_minute: None,
                updated_at: now,
            }],
            usage: vec![UsageInfo {
                scope_type: "organization".to_string(),
                scope: "default".to_string(),
                storage_bytes: 0,
                rows: None,
                updated_at: now,
            }],
        }
    }

    #[test]
    fn rejects_duplicate_database_name() {
        let meta = DatabasesMeta {
            databases: vec![db("same", "default"), db("same", "other")],
        };
        assert!(validate_database_constraints(&meta).is_err());
    }

    #[test]
    fn rejects_duplicate_management_keys() {
        let mut meta = management();
        meta.organizations.push(OrganizationInfo {
            id: "org_other".to_string(),
            name: "other".to_string(),
            slug: "default".to_string(),
            created_at: chrono::Utc::now(),
        });
        assert!(validate_management_constraints(&meta).is_err());
    }

    #[test]
    fn accepts_unique_management_keys() {
        assert!(validate_management_constraints(&management()).is_ok());
    }
}
