use axum::{routing::get, Router};

pub struct MonolithRouters {
    pub iam: Router,
    pub telegraph: Router,
    pub hive: Router,
    pub manifesto: Router,
}

pub fn compose_routes(routers: MonolithRouters) -> Router {
    Router::new()
        .route("/health", get(monolith_health))
        .nest(iam_http_server::SERVICE_PREFIX, routers.iam)
        .nest(telegraph_http_server::SERVICE_PREFIX, routers.telegraph)
        .nest(hive_http::SERVICE_PREFIX, routers.hive)
        .nest(manifesto_http_server::SERVICE_PREFIX, routers.manifesto)
}

async fn monolith_health() -> &'static str {
    "OK"
}
