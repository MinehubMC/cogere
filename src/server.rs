use crate::routes::{
    auth::{login_page, login_post},
    files,
    machine_keys::machinekeys_index,
};
use crate::{Config, auth::Backend};
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
};
use axum_login::tower_sessions::ExpiredDeletion;
use axum_login::{
    AuthManagerLayerBuilder,
    require::{RedirectHandler, Require},
    tower_sessions::SessionManagerLayer,
};
use axum_messages::MessagesManagerLayer;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::Duration;
use tokio::{net::TcpListener, signal, task::AbortHandle};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tower_sessions::Expiry;
use tower_sessions::cookie::Key;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
}

pub struct Server {
    db: SqlitePool,
    config: Config,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
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

        sqlx::migrate!().run(&pool).await.unwrap_or_else(|e| {
            eprintln!("Migration error: {e}");
            std::process::exit(1);
        });

        Ok(Self { db: pool, config })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let session_store = SqliteStore::new(self.db.clone());
        session_store.migrate().await?;

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );

        let key = Key::generate();
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(1)))
            .with_signed(key);

        let backend = Backend::new(self.db.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let require_login = Require::<Backend>::builder()
            .unauthenticated(RedirectHandler::new().login_url("/login"))
            .build();

        let state = AppState {
            db: self.db,
            config: Arc::new(self.config.clone()),
        };

        let admin_routes = Router::new()
            .route("/machine-keys", get(machinekeys_index))
            // .route("/users", get(users_index))
            .route_layer(require_login);

        let app = Router::new()
            .merge(admin_routes)
            .route("/", get(files::files_index))
            .route("/login", get(login_page))
            .route("/login", post(login_post))
            .layer(MessagesManagerLayer)
            .layer(auth_layer)
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
                    .timeout(std::time::Duration::from_secs(10))
                    .layer(TraceLayer::new_for_http())
                    .into_inner(),
            )
            .with_state(state);

        let listener = TcpListener::bind(self.config.socket_addr).await.unwrap();
        tracing::debug!("listening on {}", listener.local_addr().unwrap());

        axum::serve(listener, app.into_make_service())
            .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
            .await?;

        deletion_task.await??;

        Ok(())
    }
}

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}
