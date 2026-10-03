//! Serveur HTTP `manifesto-apparatus/1` autour de [`ReferenceKvBackend`].
//!
//! Conserve `wget` dans l'image CRI (T11). Marque le résultat d'invoke avec
//! `HOSTNAME` pour prouver que l'appel a atteint le Pod.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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
use rcgen::{CertificateParams, KeyPair};
use uuid::Uuid;

type Backend = Arc<ReferenceKvBackend<InMemoryKvStore>>;

#[derive(Clone)]
struct AppState {
    backend: Backend,
    hostname: String,
}

#[tokio::main]
async fn main() {
    enroll_from_env_or_exit().await;
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

const WORKLOAD_CERT_PATH: &str = "/tmp/lazaret-workload-cert.pem";
const WORKLOAD_KEY_PATH: &str = "/tmp/lazaret-workload-key.pem";

struct EnrollCfg {
    url: String,
    binding: Uuid,
    project_id: Uuid,
    release: String,
}

fn enroll_cfg_from_env() -> Option<EnrollCfg> {
    let url = std::env::var("LAZARET_ENROLL_URL").ok()?;
    if url.trim().is_empty() {
        return None;
    }
    let binding = env_uuid("BINDING").or_else(|| env_uuid("APPARATUS_BINDING"))?;
    let project_id = env_uuid("PROJECT_ID").or_else(|| env_uuid("APPARATUS_PROJECT"))?;
    let release = std::env::var("RELEASE")
        .ok()
        .or_else(|| std::env::var("APPARATUS_RELEASE").ok())?;
    if release.trim().is_empty() {
        return None;
    }
    Some(EnrollCfg {
        url,
        binding,
        project_id,
        release,
    })
}

fn env_uuid(name: &str) -> Option<Uuid> {
    std::env::var(name).ok().and_then(|raw| raw.parse().ok())
}

async fn enroll_from_env_or_exit() {
    let Some(cfg) = enroll_cfg_from_env() else {
        return;
    };
    if Path::new(WORKLOAD_CERT_PATH).is_file() && Path::new(WORKLOAD_KEY_PATH).is_file() {
        eprintln!("workload already enrolled on disk, skip POST /lazaret/enroll");
        return;
    }
    match enroll_workload(&cfg).await {
        Ok(()) => {}
        Err(err) if err.contains("409") => {
            eprintln!("enroll already present (409), continue serving");
        }
        Err(err) => {
            eprintln!("enroll failed: {err}");
            std::process::exit(1);
        }
    }
}

async fn enroll_workload(cfg: &EnrollCfg) -> Result<(), String> {
    let key = KeyPair::generate().map_err(|err| format!("workload key: {err}"))?;
    let params = CertificateParams::new(vec!["apparatus-reference-kv".to_owned()])
        .map_err(|err| format!("csr params: {err}"))?;
    let csr = params
        .serialize_request(&key)
        .map_err(|err| format!("csr: {err}"))?;
    let csr_pem = csr.pem().map_err(|err| format!("csr pem: {err}"))?;
    let instance = Uuid::new_v4();
    let body = serde_json::json!({
        "csr_pem": csr_pem,
        "instance": instance,
        "binding": cfg.binding,
        "release": cfg.release,
        "generation": 1,
        "grant_revision": 1,
        "project_id": cfg.project_id,
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|err| format!("http client: {err}"))?;
    let response = client
        .post(&cfg.url)
        .json(&body)
        .send()
        .await
        .map_err(|err| format!("POST enroll: {err}"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if status.as_u16() == 409 {
        return Err("409".to_owned());
    }
    if !status.is_success() {
        return Err(format!("HTTP {status} {text}"));
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|err| format!("enroll json: {err} {text}"))?;
    let cert_pem = parsed
        .get("certificate_pem")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("missing certificate_pem: {text}"))?;
    std::fs::write(WORKLOAD_CERT_PATH, cert_pem).map_err(|err| format!("write cert: {err}"))?;
    std::fs::write(WORKLOAD_KEY_PATH, key.serialize_pem())
        .map_err(|err| format!("write key: {err}"))?;
    eprintln!(
        "enroll HTTP {} written to {WORKLOAD_CERT_PATH}",
        status.as_u16()
    );
    Ok(())
}
