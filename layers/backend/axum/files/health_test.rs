// === The health endpoint, over the real router

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum_service::app;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(path: &str) -> (StatusCode, String) {
    let response = app()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("the request builds"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("the body reads")
        .to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn health_reports_ok() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"status\":\"ok\""));
}

/* The fallback exists so a wrong path fails in the same shape as every other
 * error, rather than with an empty framework default. */
#[tokio::test]
async fn an_unknown_path_returns_the_shared_error_body() {
    let (status, body) = get("/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("\"error\""));
}
