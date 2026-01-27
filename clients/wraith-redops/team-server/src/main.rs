use sqlx::postgres::PgPoolOptions;
use sqlx_core::migrate::Migrator;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

// Import generated protos
pub mod wraith {
    pub mod redops {
        tonic::include_proto!("wraith.redops");
    }
}

use wraith::redops::implant_service_server::ImplantServiceServer;
use wraith::redops::operator_service_server::OperatorServiceServer;

mod builder;
mod database;
mod governance;
mod listeners;
mod models;
mod services;
mod utils;

#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod killswitch_config_test;
#[cfg(test)]
mod operator_service_test;

use database::Database;
use governance::GovernanceEngine;
use services::implant::ImplantServiceImpl;
use services::operator::OperatorServiceImpl;
use services::session::SessionManager;
use wraith_crypto::noise::NoiseKeypair;

use std::task::{Context, Poll};
use tonic::{Request, Status};
use tower::{Layer, Service};

#[derive(Clone, Debug)]
struct RpcPath(String);

#[derive(Clone)]
struct PathLayer;

impl<S> Layer<S> for PathLayer {
    type Service = PathService<S>;

    fn layer(&self, service: S) -> Self::Service {
        PathService { service }
    }
}

#[derive(Clone)]
struct PathService<S> {
    service: S,
}

impl<S, B> Service<tonic::codegen::http::Request<B>> for PathService<S>
where
    S: Service<tonic::codegen::http::Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: tonic::codegen::http::Request<B>) -> Self::Future {
        let path = req.uri().path().to_string();
        req.extensions_mut().insert(RpcPath(path));
        self.service.call(req)
    }
}

#[allow(clippy::result_large_err)]
fn auth_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    // Whitelist Authenticate method
    if let Some(path) = req.extensions().get::<RpcPath>()
        && path.0 == "/wraith.redops.OperatorService/Authenticate"
    {
        return Ok(req);
    }

    let token = match req.metadata().get("authorization") {
        Some(t) => {
            let s = t
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid auth header"))?;
            if let Some(stripped) = s.strip_prefix("Bearer ") {
                stripped
            } else {
                return Err(Status::unauthenticated("Invalid auth scheme"));
            }
        }
        None => return Err(Status::unauthenticated("Missing authorization header")),
    };

    let claims = utils::verify_jwt(token)
        .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;

    req.extensions_mut().insert(claims);
    Ok(req)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Database connection - requires DATABASE_URL environment variable
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set (e.g., postgres://user:pass@localhost/wraith_redops)");

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run migrations (runtime-loaded to avoid sqlx umbrella's migrate feature
    // which triggers a libsqlite3-sys links conflict with workspace rusqlite)
    info!("Running database migrations...");
    Migrator::new(Path::new("./migrations"))
        .await?
        .run(&pool)
        .await?;

    let db = Arc::new(Database::new(pool));

    // Load playbooks
    if let Err(e) = services::playbook_loader::load_playbooks(db.clone()).await {
        tracing::error!("Failed to load playbooks: {}", e);
    }

    let governance = Arc::new(GovernanceEngine::new());
    let static_key = NoiseKeypair::generate()
        .map_err(|e| format!("Failed to generate static key: {:?}", e))
        .expect("Noise keypair generation failed");
    let sessions = Arc::new(SessionManager::new());

    // Event broadcast channel
    let (event_tx, _rx) = tokio::sync::broadcast::channel(100);

    let listener_manager = Arc::new(services::listener::ListenerManager::new(
        db.clone(),
        governance.clone(),
        sessions.clone(),
        Arc::new(static_key.clone()),
        event_tx.clone(),
    ));

    // Restore active listeners from database
    if let Ok(listeners) = db.list_listeners().await {
        for l in listeners {
            if l.status == "active" {
                // Determine port from environment variables or type defaults
                let port = match l.r#type.as_str() {
                    "http" => std::env::var("HTTP_LISTEN_PORT")
                        .unwrap_or_else(|_| "8080".to_string())
                        .parse()
                        .unwrap_or(8080),
                    "udp" => std::env::var("UDP_LISTEN_PORT")
                        .unwrap_or_else(|_| "9999".to_string())
                        .parse()
                        .unwrap_or(9999),
                    "dns" => std::env::var("DNS_LISTEN_PORT")
                        .unwrap_or_else(|_| "5454".to_string())
                        .parse()
                        .unwrap_or(5454),
                    "smb" => std::env::var("SMB_LISTEN_PORT")
                        .unwrap_or_else(|_| "4445".to_string())
                        .parse()
                        .unwrap_or(4445),
                    _ => 0,
                };

                if port > 0
                    && let Err(e) = listener_manager
                        .start_listener(&l.id.to_string(), &l.r#type, &l.bind_address, port)
                        .await
                {
                    tracing::error!("Failed to restart listener {}: {}", l.name, e);
                }
            }
        }
    }

    let addr_str = std::env::var("GRPC_LISTEN_ADDR")
        .expect("GRPC_LISTEN_ADDR environment variable must be set (e.g., 0.0.0.0:50051)");
    let addr: SocketAddr = addr_str.parse()?;
    let operator_service = OperatorServiceImpl {
        db: db.clone(),
        event_tx: event_tx.clone(),
        governance: governance.clone(),
        static_key: Arc::new(static_key.clone()),
        sessions: sessions.clone(),
        listener_manager: listener_manager.clone(),
    };
    let implant_service = ImplantServiceImpl {
        db: db.clone(),
        event_tx: event_tx.clone(),
    };

    info!("Team Server listening on {}", addr);

    Server::builder()
        .layer(PathLayer)
        .add_service(OperatorServiceServer::with_interceptor(
            operator_service,
            auth_interceptor,
        ))
        .add_service(ImplantServiceServer::new(implant_service))
        .serve(addr)
        .await?;

    Ok(())
}
