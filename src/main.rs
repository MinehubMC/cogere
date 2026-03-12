mod assembler;
mod auth;
mod database;
mod errors;
mod models;
mod routes;
mod server;
mod storage;

use crate::server::Server;
use std::{net::SocketAddr, path::PathBuf};
use tower_sessions::cookie::Key;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct Config {
    pub data_folder: PathBuf,
    pub socket_addr: SocketAddr,
    pub cookie_key: Key,
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

        let cookie_key = match std::env::var("COGERE_COOKIE_KEY") {
            Ok(k) if k.len() >= 64 => Key::from(k.as_bytes()),
            Ok(_) => {
                eprintln!(
                    "Warning: COGERE_COOKIE_KEY must be at least 64 bytes, generating a temporary key - sessions will not persist across restarts"
                );
                Key::generate()
            }
            Err(_) => {
                eprintln!(
                    "Warning: COGERE_COOKIE_KEY not set, generating a temporary key - sessions will not persist across restarts"
                );
                Key::generate()
            }
        };

        Ok(Self {
            data_folder,
            socket_addr,
            cookie_key,
        })
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("data_folder", &self.data_folder)
            .field("socket_addr", &self.socket_addr)
            .field("cookie_key", &"[redacted]")
            .finish()
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
