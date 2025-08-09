use axum::{middleware, Router};
use rustycog_command::GenericCommandService;
use rustycog_config::ServerConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

use crate::{
    handle_panic, health_check, jwt_handler::UserIdExtractor, middleware_auth::{auth_middleware, optional_auth_middleware},
};
use rustycog_permission::{Permission, PermissionsFetch};
use crate::middleware_permission::{PermissionGuard, permission_middleware, optional_permission_middleware};

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

    /// Add an authenticated GET route (requires user ID extraction)
    pub fn authenticated_get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::get(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                auth_middleware,
            )),
        );
        self
    }

    /// Add an authenticated POST route (requires user ID extraction)
    pub fn authenticated_post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::post(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                auth_middleware,
            )),
        );
        self
    }

    /// Add an authenticated PUT route (requires user ID extraction)
    pub fn authenticated_put<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::put(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                auth_middleware,
            )),
        );
        self
    }

    /// Add an authenticated DELETE route (requires user ID extraction)
    pub fn authenticated_delete<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::delete(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                auth_middleware,
            )),
        );
        self
    }

    /// Add an optional auth GET route (authentication is optional)
    pub fn optional_auth_get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::get(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                optional_auth_middleware,
            )),
        );
        self
    }

    /// Add an optional auth POST route (authentication is optional)
    pub fn optional_auth_post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::post(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                optional_auth_middleware,
            )),
        );
        self
    }

    /// Add an optional auth PUT route (authentication is optional)
    pub fn optional_auth_put<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::put(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                optional_auth_middleware,
            )),
        );
        self
    }

    /// Add an optional auth DELETE route (authentication is optional)
    pub fn optional_auth_delete<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: axum::handler::Handler<T, AppState>,
        T: 'static,
    {
        self.push_current();
        self.current_path = Some(path.to_string());
        self.current_layer = Some(
            axum::routing::delete(handler).route_layer(middleware::from_fn_with_state(
                self.state.user_id_extractor.clone(),
                optional_auth_middleware,
            )),
        );
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
        fetcher: Arc<dyn PermissionsFetch>,
        model_path: String,
    ) -> Self {
        let guard = Arc::new(PermissionGuard { required, fetcher, model_path });
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

    /// Attach an optional permission guard to the current route
    pub fn with_optional_permission(
        mut self,
        required: Permission,
        fetcher: Arc<dyn PermissionsFetch>,
        model_path: String,
    ) -> Self {
        let guard = Arc::new(PermissionGuard { required, fetcher, model_path });
        if let Some(layer) = self.current_layer.take() {
            self.current_layer = Some(
                layer
                    .route_layer(middleware::from_fn_with_state(
                        guard,
                        optional_permission_middleware,
                    )),
            );
        }
        self
    }
}

impl RouteBuilder {
    /// Finalize and return the Axum router without starting the server
    pub fn into_router(mut self) -> Router<AppState>
    where
        AppState: Clone + Send + Sync + 'static,
    {
        self.push_current();
        self.router
            .layer(CatchPanicLayer::custom(handle_panic))
            .with_state(self.state)
    }
}
