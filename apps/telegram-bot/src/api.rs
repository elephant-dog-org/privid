use std::sync::Arc;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
};
use serde::Serialize;
use tower_http::cors::{CorsLayer, Any};
use log::info;

use crate::registry::Registry;

#[derive(Clone)]
pub struct ApiState {
    pub registry: Arc<Registry>,
}

#[derive(Serialize)]
pub struct TwitterLookupResponse {
    pub verified: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sbt_types: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ens_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub registered_users: usize,
}

/// Look up a Twitter handle and return verification info.
async fn lookup_twitter(
    State(state): State<ApiState>,
    Path(handle): Path<String>,
) -> Result<Json<TwitterLookupResponse>, StatusCode> {
    let entry = state.registry.lookup_by_twitter_handle(&handle).await;

    match entry {
        Some(entry) if !entry.verified_sbt_types.is_empty() => {
            Ok(Json(TwitterLookupResponse {
                verified: true,
                sbt_types: entry.verified_sbt_types.iter().map(|v| v.short_name().to_string()).collect(),
                ens_name: entry.ens_name,
                wallet_address: entry.wallet_address,
            }))
        }
        Some(entry) => {
            Ok(Json(TwitterLookupResponse {
                verified: false,
                sbt_types: vec![],
                ens_name: entry.ens_name,
                wallet_address: String::new(),
            }))
        }
        None => {
            Ok(Json(TwitterLookupResponse {
                verified: false,
                sbt_types: vec![],
                ens_name: String::new(),
                wallet_address: String::new(),
            }))
        }
    }
}

/// Return basic stats.
async fn stats(State(state): State<ApiState>) -> Json<StatsResponse> {
    Json(StatsResponse {
        registered_users: state.registry.count().await,
    })
}

/// Health check endpoint.
async fn health() -> &'static str {
    "ok"
}

/// Start the HTTP API server on the given port.
pub async fn start_api_server(registry: Arc<Registry>, port: u16) {
    let state = ApiState { registry };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/twitter/{handle}", get(lookup_twitter))
        .route("/api/v1/stats", get(stats))
        .route("/health", get(health))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
