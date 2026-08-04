// === A minimal HTTP service

use actix_web::{App, HttpServer, Responder, get};

#[get("/health")]
async fn health() -> impl Responder {
    "ok"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(health))
        .bind(("0.0.0.0", 3000))?
        .run()
        .await
}
