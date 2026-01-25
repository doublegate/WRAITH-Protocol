use tonic::transport::Server;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

// Import generated protos
pub mod wraith {
    pub mod redops {
        tonic::include_proto!("wraith.redops");
    }
}

use wraith::redops::operator_service_server::OperatorServiceServer;
use wraith::redops::implant_service_server::ImplantServiceServer;

mod database;
mod services;
mod models;

use services::operator::OperatorServiceImpl;
use services::implant::ImplantServiceImpl;
use database::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/wraith_redops".to_string());

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Arc::new(Database::new(pool));
    
    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    let operator_service = OperatorServiceImpl { db: db.clone() };
    let implant_service = ImplantServiceImpl { db: db.clone() };

    info!("Team Server listening on {}", addr);

    Server::builder()
        .add_service(OperatorServiceServer::new(operator_service))
        .add_service(ImplantServiceServer::new(implant_service))
        .serve(addr)
        .await?;

    Ok(())
}
