use tokio::io;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::routes::create_routes;

mod error;
mod handlers;
mod models;
mod routes;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug,sse_axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    let router = create_routes();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:18000").await?;
    axum::serve(listener, router).await
}
