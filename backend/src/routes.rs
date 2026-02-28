use axum::{extract::DefaultBodyLimit, routing::{get, post}, Router};
use std::sync::Arc;

use crate::handlers::{health::health, demo::demo_form, judge::{judge_text, judge_json}};
use crate::state::AppState;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/demo",   get(demo_form))
        .route("/judge",  post(judge_text))
        .route("/judge/json", post(judge_json))
        .layer(DefaultBodyLimit::max(state.max_body_bytes))
        .with_state(state)
}