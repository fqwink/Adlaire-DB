pub mod admin;
pub mod health;
pub mod pipeline;

use axum::routing::{delete, get, post};

use crate::state::SharedState;

pub fn build_router(state: SharedState) -> axum::Router {
    axum::Router::new()
        .route("/v2/pipeline",          post(pipeline::handle))
        .route("/v2/health",            get(health::handle))
        // Phase 6: パスベース DB ルーティング
        .route("/:db_name/v2/pipeline", post(pipeline::handle_db))
        .with_state(state)
}

pub fn build_admin_router(state: SharedState) -> axum::Router {
    axum::Router::new()
        .nest("/admin/v1", axum::Router::new()
            .route("/databases",
                get(admin::databases::list).post(admin::databases::create))
            .route("/databases/:name",
                get(admin::databases::get).delete(admin::databases::delete))
            .route("/tokens",
                get(admin::tokens::list).post(admin::tokens::create))
            .route("/tokens/:id",
                get(admin::tokens::get).delete(admin::tokens::revoke))
            .route("/metrics", get(admin::metrics::get))
            .route("/databases/:name/backup",                get(admin::backup::backup))
            .route("/databases/:name/restore",               post(admin::backup::restore))
            .route("/databases/:name/restore/point-in-time", post(admin::backup::pitr))
            .route("/databases/:name/branches",
                get(admin::branches::list).post(admin::branches::create))
            .route("/databases/:name/branches/:branch",      delete(admin::branches::delete))
        )
        .with_state(state)
}
