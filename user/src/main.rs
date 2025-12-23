mod user;

use anyhow::Result;
use proto::user::user_service_server::UserServiceServer;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tonic::transport::Server;
use user::UserServiceImpl;

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

    let addr = "0.0.0.0:50051".parse()?;
    let user_service = UserServiceImpl::new(pool);

    println!("User service listening on {}", addr);

    Server::builder()
        .add_service(UserServiceServer::new(user_service))
        .serve(addr)
        .await?;

    Ok(())
}