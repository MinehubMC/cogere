mod auth;
mod database;
mod errors;
mod models;
mod routes;
mod server;

use crate::server::Server;
use std::{net::SocketAddr, path::PathBuf};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone)]
pub struct Config {
    pub data_folder: PathBuf,
    pub socket_addr: SocketAddr,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let data_folder = std::env::var("COGERE_DATA_FOLDER")
            .map(PathBuf::from)
            .map_err(|_| "COGERE_DATA_FOLDER is not set".to_string())?;

        let socket_addr = std::env::var("COGERE_SOCKET_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
            .parse::<SocketAddr>()
            .map_err(|e| format!("COGERE_SOCKET_ADDR is not a valid socket address: {e}"))?;

        Ok(Self {
            data_folder,
            socket_addr,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    });

    Server::new(config).await?.serve().await
}
