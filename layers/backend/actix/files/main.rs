// === Binary entry point

use actix_service::config::Config;
use actix_service::error::AppError;
use actix_service::routes;
use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    let config = Config::from_env()?;
    let address = config.address();
    println!("listening on http://{address}");
    HttpServer::new(|| App::new().configure(routes))
        .bind(address)?
        .run()
        .await?;
    Ok(())
}
