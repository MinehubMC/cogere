mod routes;

use crate::routes::files;

use axum::{Router, error_handling::HandleErrorLayer, http::StatusCode, routing::get};
use sqlx::SqlitePool;
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub data_folder: PathBuf,
    pub socket_addr: SocketAddr,
    pub run_migrations: bool,
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

        let run_migrations = std::env::var("COGERE_RUN_MIGRATIONS")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|e| format!("COGERE_RUN_MIGRATIONS must be 'true' or 'false': {e}"))?;

        Ok(Self {
            data_folder,
            socket_addr,
            run_migrations,
        })
    }
}

#[tokio::main]
async fn main() {
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

    let db_dir = config.data_folder.join(".cogere");
    if let Err(e) = std::fs::create_dir_all(&db_dir) {
        eprintln!(
            "Error: failed to create directory {}: {}",
            db_dir.display(),
            e
        );
        std::process::exit(1);
    }

    let db_path = db_dir.join("db.sqlite3");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = match SqlitePool::connect(&db_url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!(
                "Error: failed to connect to database at {}: {}",
                db_path.display(),
                e
            );
            std::process::exit(1);
        }
    };

    if config.run_migrations {
        sqlx::migrate!().run(&pool).await.unwrap_or_else(|e| {
            eprintln!("Migration error: {e}");
            std::process::exit(1);
        });
    }

    let state = AppState {
        db: pool,
        config: Arc::new(config.clone()),
    };

    let app = Router::new()
        .route("/", get(files::files_index))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {error}"),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.socket_addr)
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
