mod assembler;
mod auth;
mod database;
mod errors;
mod middleware;
mod models;
mod plugins;
mod routes;
mod server;
mod storage;

use crate::server::Server;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};
use tower_sessions::cookie::Key;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

pub const VERSION: &str = env!("COGERE_VERSION");
pub const GIT_SHA: &str = env!("COGERE_GIT_SHA");

#[derive(Clone)]
pub struct Config {
    pub data_folder: PathBuf,
    pub socket_addr: SocketAddr,
    pub cookie_key: Key,
    pub public_base_url: Url,
    pub log_ips: bool,
    pub trusted_proxy: Option<IpAddr>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let data_folder = std::env::var("COGERE_DATA_FOLDER")
            .map(PathBuf::from)
            .map_err(|_| "COGERE_DATA_FOLDER is not set".to_string())?;

        let socket_addr = std::env::var("COGERE_SOCKET_ADDR")
            .unwrap_or_else(|_| "[::]:3000".to_string())
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

        let public_base_url = std::env::var("COGERE_PUBLIC_BASE_URL")
            .map_err(|_| "COGERE_PUBLIC_BASE_URL is not set".to_string())
            .and_then(|s| {
                Url::parse(&s).map_err(|e| format!("invalid COGERE_PUBLIC_BASE_URL: {e}"))
            })?;

        let log_ips = std::env::var("COGERE_LOG_IPS")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let trusted_proxy = match std::env::var("COGERE_TRUSTED_PROXY").as_deref() {
            Ok("none") | Err(_) => None,
            Ok(s) => Some(
                s.parse::<IpAddr>()
                    .map_err(|e| format!("invalid COGERE_TRUSTED_PROXY: {e}"))?,
            ),
        };

        Ok(Self {
            data_folder,
            socket_addr,
            cookie_key,
            public_base_url,
            log_ips,
            trusted_proxy,
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
                format!("{}=info,tower_http=info", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Starting {} {} ({})",
        env!("CARGO_CRATE_NAME"),
        VERSION,
        GIT_SHA
    );

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration error: {e}");
        std::process::exit(1);
    });

    Server::new(config).await?.serve().await
}
