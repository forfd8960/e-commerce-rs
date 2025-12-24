mod user;

use anyhow::Result;
use proto::user::user_service_server::UserServiceServer;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tonic::transport::Server;
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;
use user::UserServiceImpl;
use common::logging::LoggingLayer;
use common::ratelimit::RateLimitLayer;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    let addr = "0.0.0.0:50051".parse()?;
    let user_service = UserServiceImpl::new(pool);

    info!("User service listening on {}", addr);

    let ratelimiter = RateLimitLayer::new(10, Duration::from_secs(60));

    Server::builder()
        .layer(LoggingLayer)
        .layer(ratelimiter)
        .add_service(UserServiceServer::new(user_service))
        .serve(addr)
        .await?;

    Ok(())
}
