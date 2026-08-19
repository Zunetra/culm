// === The health endpoint, over the real routes

use actix_service::routes;
use actix_web::{App, test};

#[actix_web::test]
async fn health_reports_ok() {
    let app = test::init_service(App::new().configure(routes)).await;
    let request = test::TestRequest::get().uri("/health").to_request();
    let response = test::call_service(&app, request).await;

    assert!(response.status().is_success());
    let body = test::read_body(response).await;
    assert!(String::from_utf8_lossy(&body).contains("\"status\":\"ok\""));
}

/* The default service exists so a wrong path fails in the same shape as every
 * other error, rather than with an empty framework default. */
#[actix_web::test]
async fn an_unknown_path_returns_the_shared_error_body() {
    let app = test::init_service(App::new().configure(routes)).await;
    let request = test::TestRequest::get().uri("/nope").to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status().as_u16(), 404);
    let body = test::read_body(response).await;
    assert!(String::from_utf8_lossy(&body).contains("\"error\""));
}
