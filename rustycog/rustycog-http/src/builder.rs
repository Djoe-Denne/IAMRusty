use axum::{middleware, Router};
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::propagate_header::PropagateHeaderLayer;

use crate::{
    handle_panic, health_check, jwt_handler::UserIdExtractor, middleware_auth::{auth_middleware, optional_auth_middleware},
    tracing_middleware::{tracing_middleware, X_CORRELATION_ID},
};
use rustycog_permission::{Permission, PermissionsFetcher};
use crate::middleware_permission::{PermissionGuard, permission_middleware};

/// Application state for HTTP handlers
#[derive(Clone)]
pub struct AppState {
    pub running: bool,
    /// Command service for handling commands with cross-cutting concerns
    pub command_service: Arc<GenericCommandService>,
    /// User ID extractor for authentication
    pub user_id_extractor: Arc<UserIdExtractor>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        command_service: Arc<GenericCommandService>,
        user_id_extractor: UserIdExtractor,
    ) -> Self {
        Self {
            running: false,
            command_service,
            user_id_extractor: Arc::new(user_id_extractor),
        }
    }
}

/// Fluent route builder for creating HTTP routes
pub struct RouteBuilder {
    router: Router<AppState>,
    state: AppState,
    permissions_dir: Option<std::path::PathBuf>,
    current_resource: Option<String>,
    current_permission_fetcher: Option<Arc<dyn PermissionsFetcher>>,
    current_path: Option<String>,
    current_layer: Option<axum::routing::MethodRouter<AppState>>,
    // None: no auth, Some(true): require auth, Some(false): optional auth
    pending_auth: Option<bool>,
}

impl RouteBuilder {
    /// Create a new route builder
    pub fn new(state: AppState) -> RouteBuilder {
        RouteBuilder {
            router: Router::new(),
            state,
            permissions_dir: None,
            current_resource: None,
            current_permission_fetcher: None,
            current_path: None,
            current_layer: None,
            pending_auth: None,
        }
    }

    /// Add a route with a method router
    fn push_current(&mut self) {
        if let (Some(path), Some(layer)) = (self.current_path.take(), self.current_layer.take()) {
            let mut layer = layer;
            // Apply pending auth as the outermost layer so it runs first
            if let Some(require_auth) = self.pending_auth.take() {
                layer = if require_auth {
                    layer.route_layer(middleware::from_fn_with_state(
                        self.state.user_id_extractor.clone(),
                        auth_middleware,
                    ))
                } else {
                    layer.route_layer(middleware::from_fn_with_state(
                        self.state.user_id_extractor.clone(),
                        optional_auth_middleware,
                    ))
                };
            }
            let router = std::mem::take(&mut self.router);
            self.router = router.route(&path, layer);
        }
    }

    pub fn route(mut self, path: &str, method_router: axum::routing::MethodRouter<AppState>) -> Self {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(method_router);
        self
    }

    /// Add a GET route
    pub fn get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(axum::routing::get(handler));
        self
    }

    /// Add a POST route
    pub fn post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(axum::routing::post(handler));
        self
    }

    /// Add a PUT route
    pub fn put<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(axum::routing::put(handler));
        self
    }

    /// Add a DELETE route
    pub fn delete<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(axum::routing::delete(handler));
        self
    }

    /// Add a health check endpoint
    pub fn health_check(mut self) -> Self {
        self.push_current();
        self.router = self.router.route("/health", axum::routing::get(health_check));
        self
    }

    /// Add nested routes with a prefix
    pub fn nest(mut self, prefix: &str, router: Router<AppState>) -> Self {
        self.router = self.router.nest(prefix, router);
        self
    }

    /// Build the final router with panic handling
    pub async fn build(mut self, config: ServerConfig) -> anyhow::Result<()>
    where
        AppState: Clone + Send + Sync + 'static,
    {
        // Push any pending route being built
        self.push_current();

        let app = self
            .router
            .layer(CatchPanicLayer::custom(handle_panic))
            .layer(PropagateHeaderLayer::new(X_CORRELATION_ID.parse().unwrap()))
            .layer(middleware::from_fn(tracing_middleware))
            .with_state(self.state);

        if config.tls_enabled {
            tracing::info!(
                "Starting HTTPS server on {}:{}",
                config.host,
                config.tls_port
            );

            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                config.tls_cert_path,
                config.tls_key_path,
            )
            .await?;
            let addr: SocketAddr = format!("{}:{}", config.host, config.tls_port).parse()?;

            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        } else {
            tracing::info!("Starting HTTP server on {}:{}", config.host, config.port);
            let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }

        Ok(())
    }
}

impl RouteBuilder {

    fn get_model_path(&self) -> String {
        if self.permissions_dir.is_none() {
            panic!("Permissions directory not set");
        }
        let model_path = self.permissions_dir.clone().unwrap().join(format!("{}.conf", self.current_resource.clone().unwrap()));
        if !model_path.exists() {
            panic!("Model file {} does not exist", model_path.to_string_lossy());
        }
        model_path.to_string_lossy().to_string()
    }

    pub fn permissions_dir(mut self, dir: std::path::PathBuf) -> Self {
        //check if dir exists, throw DomainError if not
        if !dir.exists() {
            panic!("Permissions directory does not exist for path: {} from current working directory: {}", dir.to_string_lossy(), std::env::current_dir().unwrap().to_string_lossy());
        }
        self.permissions_dir = Some(dir);
        self
    }

    pub fn resource(mut self, resource: &str) -> Self {
        self.current_resource = Some(resource.to_string());
        self
    }

    pub fn with_permission_fetcher(mut self, fetcher: Arc<dyn PermissionsFetcher>) -> Self {
        self.current_permission_fetcher = Some(fetcher);
        self
    }
}

impl RouteBuilder {
    /// Mark the current route as requiring authentication
    pub fn authenticated(mut self) -> Self {
        self.pending_auth = Some(true);
        self
    }

    /// Mark the current route as allowing optional authentication
    pub fn might_be_authenticated(mut self) -> Self {
        self.pending_auth = Some(false);
        self
    }

    /// Attach a permission guard to the current route
    pub fn with_permission(
        mut self,
        required: Permission,
    ) -> Self {
        let model_path = self.get_model_path();
        let guard = Arc::new(PermissionGuard { required, 
            fetcher: self.current_permission_fetcher.clone().unwrap(), 
            model_path,
        });
        if let Some(layer) = self.current_layer.take() {
            self.current_layer = Some(
                layer
                    .route_layer(middleware::from_fn_with_state(
                        guard,
                        permission_middleware,
                    )),
            );
        }
        self
    }
}
