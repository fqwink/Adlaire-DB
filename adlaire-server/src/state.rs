use std::sync::Arc;

use crate::{auth::AuthState, config::Config, db::DbManager, metrics::Metrics};

#[derive(Clone)]
pub struct AppState {
    pub config:      Arc<Config>,
    pub db_mgr:      Arc<DbManager>,
    pub auth:        Arc<AuthState>,
    pub metrics:     Arc<Metrics>,
    pub role:        ServerRole,
    pub replication: Option<Arc<()>>, // Phase 10 で ReplicationState に差し替え
}

pub type SharedState = Arc<AppState>;

#[derive(Clone, Debug, PartialEq)]
pub enum ServerRole {
    Standalone,
    Primary { primary_port: u16 },
    // Phase 10: Replica { primary_url: url::Url },
}
