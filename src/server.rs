use crate::{
    Config,
    assembler::{job::AssemblyJob, worker},
    auth::{admin::require_admin, auth::Backend},
    database::settings::load_instance_settings,
    errors::Error,
    models::settings::InstanceSettings,
    routes::{
        admin, assembler, assets,
        auth::{login_page, login_post},
        files, groups, plugins,
    },
    storage::filesystem::FilesystemStorage,
};
use axum::{
    Router,
    error_handling::HandleErrorLayer,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{delete, get, post},
};
use axum_login::tower_sessions::ExpiredDeletion;
use axum_login::{
    AuthManagerLayerBuilder,
    require::{RedirectHandler, Require},
    tower_sessions::SessionManagerLayer,
};
use axum_messages::MessagesManagerLayer;
use sqlx::SqlitePool;
use std::sync::{Arc, atomic::AtomicUsize};
use time::Duration;
use tokio::{
    net::TcpListener,
    signal,
    sync::{RwLock, mpsc},
    task::AbortHandle,
};
use tower::{BoxError, ServiceBuilder};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};
use tower_sessions::Expiry;
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub storage: FilesystemStorage,
    pub settings: Arc<RwLock<InstanceSettings>>,
    pub assembly_tx: mpsc::Sender<AssemblyJob>,
    pub active_assembly_jobs: Arc<AtomicUsize>,
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

        let key = self.config.cookie_key.clone();
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(1)))
            .with_signed(key);

        let backend = Backend::new(self.db.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        let require_login = Require::<Backend>::builder()
            .unauthenticated(RedirectHandler::new().login_url("/login"))
            .build();

        let settings = load_instance_settings(&self.db).await?;

        const ASSEMBLY_QUEUE_SIZE: usize = 100;

        let (assembly_tx, assembly_rx) = mpsc::channel::<AssemblyJob>(ASSEMBLY_QUEUE_SIZE);

        let state = AppState {
            db: self.db,
            config: Arc::new(self.config.clone()),
            storage: FilesystemStorage::new(self.config.data_folder),
            settings: Arc::new(RwLock::new(settings)),
            assembly_tx,
            active_assembly_jobs: Arc::new(AtomicUsize::new(0)),
        };

        tokio::spawn(worker::run(
            assembly_rx,
            state.db.clone(),
            state.settings.clone(),
            state.storage.clone(),
            state.active_assembly_jobs.clone(),
        ));

        tokio::spawn(crate::assembler::cleanup::run(
            state.db.clone(),
            state.storage.clone(),
            state.settings.clone(),
        ));

        let admin_routes = Router::new()
            .route("/admin/settings", get(admin::settings_index))
            .route("/admin/settings/reload", post(admin::settings_reload))
            .route_layer(middleware::from_fn(require_admin));

        let authenticated_routes = Router::new()
            .route(
                "/groups",
                get(groups::groups_index).post(groups::create_group),
            )
            .route("/g/{group_id}", get(groups::groups_detail))
            .route(
                "/g/{group_id}/machine-keys",
                get(groups::group_machine_keys).post(groups::create_group_machine_key),
            )
            .route(
                "/g/{group_id}/machine-keys/{key_id}",
                delete(groups::delete_group_machine_key),
            )
            .route(
                "/g/{group_id}/machine-keys/{key_id}/permissions",
                post(groups::add_group_machine_key_permission)
                    .delete(groups::remove_group_machine_key_permission),
            )
            .route("/g/{group_id}/members", get(groups::groups_members))
            .route("/g/{group_id}/plugins", get(groups::groups_plugins))
            .route(
                "/api/v1/groups/{group_id}/assemble",
                post(assembler::request_assembly),
            )
            .route(
                "/api/v1/groups/{group_id}/assemblies/{id}",
                get(assembler::get_assembly),
            )
            .route(
                "/api/v1/groups/{group_id}/assemblies/{id}/download",
                get(assembler::download_assembly),
            )
            .route(
                "/api/v1/groups/{group_id}/plugins",
                post(plugins::plugin_upload),
            )
            .merge(admin_routes)
            .route_layer(require_login);

        let app = Router::new()
            .merge(authenticated_routes)
            .route("/", get(files::files_index))
            .route("/assets/{*path}", get(assets::serve_asset))
            .route("/login", get(login_page).post(login_post))
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
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(
                250 * 1024 * 1024, // 250Mb
            ))
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

pub async fn reload_settings(state: &AppState) -> Result<(), Error> {
    let fresh = load_instance_settings(&state.db).await?;
    *state.settings.write().await = fresh;
    Ok(())
}
