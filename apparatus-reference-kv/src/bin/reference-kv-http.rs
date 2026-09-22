//! Serveur HTTP `manifesto-apparatus/1` autour de [`ReferenceKvBackend`].
//!
//! Conserve `wget` dans l'image CRI (T11). Marque le résultat d'invoke avec
//! `HOSTNAME` pour prouver que l'appel a atteint le Pod.

use std::net::SocketAddr;
use std::sync::Arc;

use apparatus_contracts::{
    digest_str, new_operation_id, BindRequest, BindResponse, ConfigureRequest, ConfigureResponse,
    HealthResponse, HealthStatus, InvokeRequest, InvokeResponse, ReadyResponse, UnbindRequest,
    UnbindResponse, BIND_PATH, CONFIGURE_PATH, HEALTH_PATH, INVOKE_PATH, READY_PATH, UNBIND_PATH,
};
use apparatus_reference_kv::{
    InMemoryKvStore, ReferenceKvBackend, REFERENCE_APPARATUS_ID, REFERENCE_VERSION,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};

type Backend = Arc<ReferenceKvBackend<InMemoryKvStore>>;

#[derive(Clone)]
struct AppState {
    backend: Backend,
    hostname: String,
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(8080);
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "reference-kv".to_owned());
    let state = AppState {
        backend: Arc::new(ReferenceKvBackend::new(InMemoryKvStore::new())),
        hostname,
    };
    let app = Router::new()
        .route(HEALTH_PATH, get(health))
        .route(READY_PATH, get(ready))
        .route(BIND_PATH, post(bind))
        .route(CONFIGURE_PATH, post(configure))
        .route(INVOKE_PATH, post(invoke))
        .route(UNBIND_PATH, post(unbind))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|err| panic!("bind {addr}: {err}"));
    axum::serve(listener, app)
        .await
        .unwrap_or_else(|err| panic!("serve: {err}"));
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: HealthStatus::Ok,
        apparatus_id: REFERENCE_APPARATUS_ID.parse().expect("id référence"),
    })
}

async fn ready() -> Json<ReadyResponse> {
    Json(ReadyResponse {
        ready: true,
        apparatus_id: REFERENCE_APPARATUS_ID.parse().expect("id référence"),
        release_digest: digest_str(REFERENCE_VERSION),
    })
}

async fn bind(
    State(state): State<AppState>,
    Json(request): Json<BindRequest>,
) -> Result<Json<BindResponse>, (StatusCode, String)> {
    state
        .backend
        .bind(&request)
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

async fn configure(
    State(state): State<AppState>,
    Json(request): Json<ConfigureRequest>,
) -> Result<Json<ConfigureResponse>, (StatusCode, String)> {
    state
        .backend
        .configure(&request)
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

async fn invoke(
    State(state): State<AppState>,
    Json(request): Json<InvokeRequest>,
) -> Result<Json<InvokeResponse>, (StatusCode, String)> {
    ensure_bound(&state, &request)?;
    let mut response = state
        .backend
        .invoke(&request)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    stamp_hostname(&mut response, &state.hostname);
    Ok(Json(response))
}

async fn unbind(
    State(state): State<AppState>,
    Json(request): Json<UnbindRequest>,
) -> Result<Json<UnbindResponse>, (StatusCode, String)> {
    state
        .backend
        .unbind(&request)
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

fn ensure_bound(state: &AppState, request: &InvokeRequest) -> Result<(), (StatusCode, String)> {
    if state.backend.is_bound(&request.binding_id) {
        return Ok(());
    }
    let bind = BindRequest {
        binding_id: request.binding_id.clone(),
        apparatus_id: REFERENCE_APPARATUS_ID.parse().expect("id référence"),
        release_digest: digest_str(REFERENCE_VERSION),
        operation_id: new_operation_id(),
        config: None,
    };
    state
        .backend
        .bind(&bind)
        .map(|_| ())
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

fn stamp_hostname(response: &mut InvokeResponse, hostname: &str) {
    match &mut response.result {
        serde_json::Value::Object(map) => {
            map.insert(
                "hostname".to_owned(),
                serde_json::Value::String(hostname.to_owned()),
            );
        }
        other => {
            response.result = serde_json::json!({
                "value": other,
                "hostname": hostname,
            });
        }
    }
}
