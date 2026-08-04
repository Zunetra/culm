// === A minimal HTTP service

use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("port 3000 is free");
    axum::serve(listener, app).await.expect("the server runs");
}

/// Answers the liveness probe.
async fn health() -> &'static str {
    "ok"
}
