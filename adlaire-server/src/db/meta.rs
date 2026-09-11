use std::path::Path;

use super::DbInfo;

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
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}
