mod api;
mod store;

use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("yaya_coordinator=info".parse()?),
        )
        .init();

    let db_path = std::env::var("YAYA_DB_PATH").unwrap_or_else(|_| "yaya-coordinator.db".into());
    let listen = std::env::var("YAYA_LISTEN").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let store = Arc::new(store::Store::open(&db_path)?);
    tracing::info!(db = %db_path, "Database initialized");

    let app = api::router(store);

    let listener = tokio::net::TcpListener::bind(&listen).await?;
    tracing::info!(listen = %listen, "Yaya coordinator listening");

    axum::serve(listener, app).await?;
    Ok(())
}
