use axum::{
    Router,
    routing::{delete, get, post},
};
use std::time::Duration;

use crate::api::handlers::{document, folder, health, index, search, settings, suggest};
use crate::api::middlewares::{
    DEFAULT_TIMEOUT_DURATION, apply_middlewares, create_compression_layer,
};
use crate::state::AppState;

pub fn create_router_with_timeout(state: AppState, timeout_duration: Duration) -> Router {
    // Heavy payload routes with selective Gzip compression
    let heavy_routes = Router::new()
        .route("/api/search", get(search::search))
        .route("/api/stats", get(health::stats))
        .route(
            "/api/documents/{id}",
            get(document::get_document).delete(document::delete_document),
        )
        .layer(create_compression_layer());

    let core_router = Router::new()
        // System Health & Observability
        .route("/api/health", get(health::health_summary))
        .route("/api/health/live", get(health::health_live))
        .route("/api/health/ready", get(health::health_ready))
        // Suggestion Engine
        .route("/api/suggest", get(suggest::suggest))
        // Folder Management
        .route("/api/folders", get(folder::list_folders))
        .route("/api/folders/{id}", delete(folder::delete_folder))
        // Indexing & Job Operations
        .route("/api/index/folder", post(index::index_folder))
        .route("/api/index", post(index::index_document))
        .route("/api/index/jobs/{id}", get(index::get_job))
        .route("/api/index/jobs/{id}/cancel", post(index::cancel_job))
        .route("/api/index/rebuild", post(index::rebuild_index))
        // System Settings
        .route(
            "/api/settings",
            get(settings::get_settings).put(settings::update_settings),
        )
        .merge(heavy_routes);

    apply_middlewares(core_router, timeout_duration).with_state(state)
}

pub fn create_router_with_state(state: AppState) -> Router {
    create_router_with_timeout(state, DEFAULT_TIMEOUT_DURATION)
}

pub fn create_router() -> Router {
    let (state, _rx) = AppState::test_state();
    create_router_with_state(state)
}
