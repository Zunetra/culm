// === Binary entry point

use axum_service::config::Config;
use axum_service::error::AppError;
use axum_service::app;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Config::from_env()?;
    let address = config.address();
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!("listening on http://{address}");
    axum::serve(listener, app()).await?;
    Ok(())
}
