use axum::{http::Method, Router};
use tower_http::cors::{Any, CorsLayer};

mod state;
mod models;
mod routes;
mod handlers;
mod scoring;

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app_state = state::AppState::new(120, 1 * 1024 * 1024); // max_line_len, max_body_bytes
    let app = routes::build_router(app_state).layer(cors);

    println!(" :) Rust server running at http://localhost:3000");
    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap(),
        app,
    )
    .await
    .unwrap();
}