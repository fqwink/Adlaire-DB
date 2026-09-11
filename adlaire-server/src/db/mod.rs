pub mod manager;
pub mod meta;
pub mod sqld_adapter;

pub use manager::DbManager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbInfo {
    pub id:         String, // UUID v4
    pub name:       String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
}
