mod order;

use anyhow::Result;
use order::OrderServiceImpl;
use proto::order::order_service_server::OrderServiceServer;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let user_service_url =
        env::var("USER_SERVICE_URL").unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
    let product_service_url =
        env::var("PRODUCT_SERVICE_URL").unwrap_or_else(|_| "http://127.0.0.1:50052".to_string());

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to database");

    // Run migrations
    sqlx::migrate!("../migrations").run(&pool).await?;
    println!("Migrations completed");

    let addr = "0.0.0.0:50053".parse()?;
    let order_service = OrderServiceImpl::new(pool, user_service_url, product_service_url);

    println!("Order service listening on {}", addr);

    Server::builder()
        .add_service(OrderServiceServer::new(order_service))
        .serve(addr)
        .await?;

    Ok(())
}