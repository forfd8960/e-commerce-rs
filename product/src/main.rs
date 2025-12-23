mod product;

use anyhow::Result;
use proto::product::product_service_server::ProductServiceServer;
use product::ProductServiceImpl;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to database");

    let addr = "0.0.0.0:50052".parse()?;
    let product_service = ProductServiceImpl::new(pool);

    println!("Product service listening on {}", addr);

    Server::builder()
        .add_service(ProductServiceServer::new(product_service))
        .serve(addr)
        .await?;

    Ok(())
}