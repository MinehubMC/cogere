use crate::{
    Config,
    assembler::{job::AssemblyJob, worker},
    auth::auth::Backend,
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
use governor::DefaultKeyedRateLimiter;
use sqlx::SqlitePool;
use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicUsize},
};
use time::Duration;
use tokio::{
    net::TcpListener,
    signal,
    sync::{RwLock, mpsc},
    task::AbortHandle,
};
use tower::{BoxError, ServiceBuilder};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
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
            .route_layer(middleware::from_fn(
                crate::middleware::require_admin::require_admin,
            ));

        let assemble_conf = Box::new(
            GovernorConfigBuilder::default()
                .key_extractor(crate::middleware::ratelimit::EntityKeyExtractor)
                .per_second(30)
                .burst_size(3)
                .finish()
                .unwrap(),
        );
        spawn_limiter_cleanup("assemble".to_string(), assemble_conf.limiter().clone());
        let assemble_limiter = GovernorLayer::new(assemble_conf);

        let assemble_routes = Router::new()
            .route(
                "/api/v1/groups/{group_id}/assemble",
                post(assembler::request_assembly),
            )
            .route_layer(assemble_limiter);

        let download_conf = Box::new(
            GovernorConfigBuilder::default()
                .key_extractor(crate::middleware::ratelimit::EntityKeyExtractor)
                .per_second(2)
                .burst_size(20)
                .finish()
                .unwrap(),
        );
        spawn_limiter_cleanup("download".to_string(), download_conf.limiter().clone());
        let download_limiter = GovernorLayer::new(download_conf);

        let download_routes = Router::new()
            .route(
                "/api/v1/groups/{group_id}/assemblies/{id}/download",
                get(assembler::download_assembly),
            )
            .route_layer(download_limiter);

        let general_conf = Box::new(
            GovernorConfigBuilder::default()
                .key_extractor(crate::middleware::ratelimit::EntityKeyExtractor)
                .per_second(1)
                .burst_size(10)
                .finish()
                .unwrap(),
        );
        spawn_limiter_cleanup("general".to_string(), general_conf.limiter().clone());
        let general_limiter = GovernorLayer::new(general_conf);

        let ui_routes = Router::new()
            .route("/", get(files::files_index))
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
            .route_layer(general_limiter.clone())
            .merge(admin_routes)
            .route_layer(require_login);

        let api_routes = Router::new()
            .route(
                "/api/v1/groups/{group_id}/assemblies/{id}",
                get(assembler::get_assembly),
            )
            .route(
                "/api/v1/groups/{group_id}/plugins",
                post(plugins::plugin_upload),
            )
            .route_layer(general_limiter)
            .merge(assemble_routes)
            .merge(download_routes);

        let app = Router::new()
            .merge(ui_routes)
            .merge(api_routes)
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
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(move |request: &axum::http::Request<_>| {
                                let ip_str = if self.config.log_ips {
                                    request
                                        .extensions()
                                        .get::<crate::middleware::client_ip::ClientIp>()
                                        .map(|ci| ci.0.to_string())
                                        .unwrap_or_else(|| "unavailable".to_string())
                                } else {
                                    "-".to_string()
                                };

                                tracing::info_span!(
                                    "request",
                                    method = %request.method(),
                                    uri = %request.uri().path(),
                                    ip = %ip_str,
                                )
                            })
                            .on_request(())
                            .on_response(
                                tower_http::trace::DefaultOnResponse::new()
                                    .level(tracing::Level::INFO)
                                    .latency_unit(tower_http::LatencyUnit::Millis),
                            ),
                    )
                    .into_inner(),
            )
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(
                250 * 1024 * 1024, // 250Mb
            ))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::client_ip::resolve_client_ip,
            ))
            .with_state(state);

        let listener = TcpListener::bind(self.config.socket_addr).await.unwrap();
        tracing::debug!("listening on {}", listener.local_addr().unwrap());

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await?;

        match deletion_task.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("Session deletion task error: {e}"),
            Err(e) if e.is_cancelled() => {} // normal shutdown
            Err(e) => tracing::warn!("Session deletion task panicked: {e}"),
        }

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

fn spawn_limiter_cleanup<K>(name: String, limiter: Arc<DefaultKeyedRateLimiter<K>>)
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
{
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            tracing::info!("rate limiter '{}' storage size: {}", name, limiter.len());
            limiter.retain_recent();
        }
    });
}
