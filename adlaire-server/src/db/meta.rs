use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
};

use super::DbInfo;

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
        let path = data_dir.join("meta").join("databases.json");
        let tmp  = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self)?;
        atomic_write_json(data_dir, &tmp, &path, &json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrganizationInfo {
    pub id:         String,
    pub name:       String,
    pub slug:       String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupInfo {
    pub id:                String,
    pub organization:      String,
    pub name:              String,
    pub slug:              String,
    pub location:          String,
    pub delete_protection: bool,
    pub created_at:        chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocationInfo {
    pub id:         String,
    pub name:       String,
    pub provider:   String,
    pub region:     String,
    pub primary:    bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuotaInfo {
    pub scope_type:           String,
    pub scope:                String,
    pub storage_bytes:        Option<u64>,
    pub rows:                 Option<u64>,
    pub write_ops_per_minute: Option<u64>,
    pub updated_at:           chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UsageInfo {
    pub scope_type:    String,
    pub scope:         String,
    pub storage_bytes: u64,
    pub rows:          Option<u64>,
    pub updated_at:    chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ManagementMeta {
    pub organizations: Vec<OrganizationInfo>,
    pub groups:        Vec<GroupInfo>,
    pub locations:     Vec<LocationInfo>,
    pub quotas:        Vec<QuotaInfo>,
    pub usage:         Vec<UsageInfo>,
}

impl ManagementMeta {
    pub fn load_or_init(data_dir: &Path) -> anyhow::Result<Self> {
        let now = chrono::Utc::now();
        let mut organizations: Vec<OrganizationInfo> = load_list(data_dir, "organizations.json", "organizations")?;
        let mut groups: Vec<GroupInfo> = load_list(data_dir, "groups.json", "groups")?;
        let mut locations: Vec<LocationInfo> = load_list(data_dir, "locations.json", "locations")?;
        let mut quotas: Vec<QuotaInfo> = load_list(data_dir, "quotas.json", "quotas")?;
        let mut usage: Vec<UsageInfo> = load_list(data_dir, "usage.json", "usage")?;

        if organizations.is_empty() {
            organizations.push(OrganizationInfo {
                id:         default_id("org"),
                name:       "default".to_string(),
                slug:       "default".to_string(),
                created_at: now,
            });
        }
        if locations.is_empty() {
            locations.push(LocationInfo {
                id:         default_id("loc"),
                name:       "default".to_string(),
                provider:   "self-hosted".to_string(),
                region:     "local".to_string(),
                primary:    true,
                created_at: now,
            });
        }
        if groups.is_empty() {
            groups.push(GroupInfo {
                id:                default_id("grp"),
                organization:      "default".to_string(),
                name:              "default".to_string(),
                slug:              "default".to_string(),
                location:          "default".to_string(),
                delete_protection: false,
                created_at:        now,
            });
        }
        if quotas.is_empty() {
            quotas.push(QuotaInfo {
                scope_type:           "organization".to_string(),
                scope:                "default".to_string(),
                storage_bytes:        Some(10 * 1024 * 1024 * 1024),
                rows:                 None,
                write_ops_per_minute: None,
                updated_at:           now,
            });
        }
        if usage.is_empty() {
            usage.push(UsageInfo {
                scope_type:    "organization".to_string(),
                scope:         "default".to_string(),
                storage_bytes: 0,
                rows:          None,
                updated_at:    now,
            });
        }

        let meta = Self { organizations, groups, locations, quotas, usage };
        meta.save(data_dir)?;
        Ok(meta)
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        save_list(data_dir, "organizations.json", "organizations", &self.organizations)?;
        save_list(data_dir, "groups.json", "groups", &self.groups)?;
        save_list(data_dir, "locations.json", "locations", &self.locations)?;
        save_list(data_dir, "quotas.json", "quotas", &self.quotas)?;
        save_list(data_dir, "usage.json", "usage", &self.usage)?;
        Ok(())
    }
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

fn atomic_write_json(
    data_dir: &Path,
    tmp: &Path,
    path: &Path,
    json: &[u8],
) -> anyhow::Result<()> {
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
    let dir = OpenOptions::new().read(true).open(meta_dir)?;
    dir.sync_all()?;
    Ok(())
}
