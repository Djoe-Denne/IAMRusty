//! Observer Manifesto `desired_state` (M4) — hors `Manifesto/*/src`.
//!
//! Lit les 5 routes gelées `D-0006E` (`GET /api/projects/{id}/components`).
//! Ready = `status == "active"` **et** digest `sha256:` + 64 hex. Sinon skip
//! fail-closed (pas de schedule). Le ticker P2 n'est pas un controller K8s.

use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use uuid::Uuid;

use crate::admission::{
    admission_store_path_from_env, would_schedule, AdmissionStore, PersistentAdmissionStore,
};
use crate::controller::{
    run_watch, ControllerError, IsolationLabels, ReconcileOutcome, ScheduleRefuse,
    WorkloadReconciler,
};
use crate::digest::ReleaseDigest;

/// Base URL Manifesto (`http://host:port` ou déjà suffixée `/manifesto`).
pub const MANIFESTO_BASE_URL_ENV: &str = "APPARATUS_MANIFESTO_BASE_URL";
/// `project_id` observé (UUID), obligatoire si [`MANIFESTO_BASE_URL_ENV`] est set.
pub const MANIFESTO_PROJECT_ID_ENV: &str = "APPARATUS_MANIFESTO_PROJECT_ID";
/// Bearer optionnel pour `GET /components`.
pub const MANIFESTO_BEARER_ENV: &str = "APPARATUS_MANIFESTO_BEARER";
/// Intervalle de poll (secondes). Défaut : 30. Ignoré si l'URL est unset.
pub const MANIFESTO_POLL_INTERVAL_SECS_ENV: &str = "APPARATUS_MANIFESTO_POLL_INTERVAL_SECS";

const MANIFESTO_PREFIX: &str = "/manifesto";
const DEFAULT_POLL_SECS: u64 = 30;

/// Binding Manifesto ready à réconcilier (ADR-0001 : binding = `ProjectComponent.id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyBinding {
    /// Isolation label projet.
    pub project_id: Uuid,
    /// Isolation = binding (`ProjectComponent.id`).
    pub component_id: Uuid,
    /// Digest descripteur 0002 (`apparatus_bindings.digest` / champ optionnel HTTP).
    pub digest: ReleaseDigest,
}

/// Source d'observation du `desired_state` (HTTP ou SQL hors crate Manifesto).
pub trait DesiredStateSource: Send + Sync {
    /// Liste les bindings ready (active + digest parseable). Les autres sont skip.
    ///
    /// # Errors
    ///
    /// Transport HTTP ou JSON illisible (un item non-ready n'est pas une erreur).
    fn list_ready(&self)
        -> impl Future<Output = Result<Vec<ReadyBinding>, ControllerError>> + Send;
}

/// Client HTTP des 5 routes `/components` (preuve M4 + prod).
#[derive(Clone)]
pub struct HttpComponentsClient {
    http: reqwest::Client,
    base: String,
    project_id: Uuid,
    bearer: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestoComponentList {
    #[serde(default)]
    data: Vec<ManifestoComponent>,
}

#[derive(Debug, Deserialize)]
struct ManifestoComponent {
    id: Uuid,
    #[serde(default)]
    status: String,
    #[serde(default)]
    digest: Option<String>,
    /// Présent côté stub / futur SQL ; ignoré pour le filtre HTTP ready.
    #[serde(default)]
    #[allow(dead_code)]
    desired_generation: Option<i64>,
}

impl HttpComponentsClient {
    /// Construit un client. Joint `/manifesto` une fois si absent de `base_url`.
    ///
    /// # Errors
    ///
    /// Client HTTP inconstructible.
    pub fn new(
        base_url: impl Into<String>,
        project_id: Uuid,
        bearer: Option<String>,
    ) -> Result<Self, ControllerError> {
        let http = reqwest::Client::builder()
            .build()
            .map_err(|err| ControllerError::new(format!("manifesto HTTP client: {err}")))?;
        Ok(Self {
            http,
            base: normalize_manifesto_base(&base_url.into()),
            project_id,
            bearer,
        })
    }

    /// `None` si [`MANIFESTO_BASE_URL_ENV`] unset ou vide (watch-only).
    ///
    /// # Errors
    ///
    /// URL set mais `project_id` absent / UUID invalide.
    pub fn from_env() -> Result<Option<Self>, ControllerError> {
        match std::env::var(MANIFESTO_BASE_URL_ENV) {
            Err(_) => Ok(None),
            Ok(url) if url.is_empty() => Ok(None),
            Ok(url) => {
                let project = std::env::var(MANIFESTO_PROJECT_ID_ENV).map_err(|_| {
                    ControllerError::new(format!(
                        "{MANIFESTO_PROJECT_ID_ENV} required when {MANIFESTO_BASE_URL_ENV} is set"
                    ))
                })?;
                let project_id = Uuid::parse_str(project.trim()).map_err(|err| {
                    ControllerError::new(format!("{MANIFESTO_PROJECT_ID_ENV}: {err}"))
                })?;
                let bearer = std::env::var(MANIFESTO_BEARER_ENV)
                    .ok()
                    .map(|token| token.trim().to_owned())
                    .filter(|token| !token.is_empty());
                Ok(Some(Self::new(url, project_id, bearer)?))
            }
        }
    }

    async fn fetch_ready(&self) -> Result<Vec<ReadyBinding>, ControllerError> {
        let url = format!("{}/api/projects/{}/components", self.base, self.project_id);
        let mut req = self.http.get(&url);
        if let Some(token) = &self.bearer {
            req = req.bearer_auth(token);
        }
        let response = req
            .send()
            .await
            .map_err(|err| ControllerError::new(format!("manifesto GET {url}: {err}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ControllerError::new(format!(
                "manifesto GET {url}: HTTP {status}"
            )));
        }
        let list: ManifestoComponentList = response
            .json()
            .await
            .map_err(|err| ControllerError::new(format!("manifesto GET {url} JSON: {err}")))?;
        Ok(list
            .data
            .into_iter()
            .filter_map(|item| ready_binding(self.project_id, &item))
            .collect())
    }
}

impl DesiredStateSource for HttpComponentsClient {
    fn list_ready(
        &self,
    ) -> impl Future<Output = Result<Vec<ReadyBinding>, ControllerError>> + Send {
        self.fetch_ready()
    }
}

fn ready_binding(project_id: Uuid, item: &ManifestoComponent) -> Option<ReadyBinding> {
    if item.status != "active" {
        return None;
    }
    let digest = item
        .digest
        .as_deref()
        .and_then(|raw| ReleaseDigest::new(raw).ok())?;
    Some(ReadyBinding {
        project_id,
        component_id: item.id,
        digest,
    })
}

fn normalize_manifesto_base(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.ends_with(MANIFESTO_PREFIX) {
        trimmed.to_owned()
    } else {
        format!("{trimmed}{MANIFESTO_PREFIX}")
    }
}

/// Pont `desired_state` → [`WorkloadReconciler`] (M3).
pub struct DesiredStateBridge;

impl DesiredStateBridge {
    /// 1. `list_ready` 2. `reconcile_digest` 3. apply CR seulement si M3 refuse
    /// (`MissingAdmissionRecord` / `MissingIsolation`) **et** JSON VALID+pin.
    ///
    /// Digest non VALID : `reconcile_digest` seul (refus M3, zéro Pod).
    /// Re-poll : CR déjà présente → pas de delete+create (`apply_valid_record`).
    ///
    /// # Errors
    ///
    /// Lecture source, isolation vide, apply CR, ou API kube.
    pub async fn reconcile_ready<S: DesiredStateSource>(
        source: &S,
        reconciler: &WorkloadReconciler,
        store: &PersistentAdmissionStore,
    ) -> Result<Vec<ReconcileOutcome>, ControllerError> {
        let bindings = source.list_ready().await?;
        let mut outcomes = Vec::with_capacity(bindings.len());
        for binding in bindings {
            let line = store.get(&binding.digest);
            let cri_image = line.as_ref().and_then(|record| record.cri_image.clone());
            let mut outcome = reconciler.reconcile_digest(&binding.digest).await?;
            let needs_apply = matches!(
                outcome,
                ReconcileOutcome::Refused(
                    ScheduleRefuse::MissingAdmissionRecord | ScheduleRefuse::MissingIsolation
                )
            );
            if needs_apply && would_schedule(store, &binding.digest) {
                let isolation = IsolationLabels::try_new(
                    binding.project_id.to_string(),
                    binding.component_id.to_string(),
                )?;
                if let (Some(record), Some(cri_image)) = (line.as_ref(), cri_image.as_deref()) {
                    reconciler
                        .apply_valid_record(record, cri_image, Some(&isolation))
                        .await?;
                    outcome = reconciler.reconcile_digest(&binding.digest).await?;
                }
            }
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }
}

/// Alias de [`DesiredStateBridge::reconcile_ready`].
///
/// # Errors
///
/// Voir [`DesiredStateBridge::reconcile_ready`].
pub async fn reconcile_ready<S: DesiredStateSource>(
    source: &S,
    reconciler: &WorkloadReconciler,
    store: &PersistentAdmissionStore,
) -> Result<Vec<ReconcileOutcome>, ControllerError> {
    DesiredStateBridge::reconcile_ready(source, reconciler, store).await
}

/// Watch CR (T10). Poll Manifesto en parallèle si [`MANIFESTO_BASE_URL_ENV`] est set.
///
/// # Errors
///
/// Client kube, store JSON, ou config Manifesto incomplète.
pub async fn run_controller() -> Result<(), ControllerError> {
    match HttpComponentsClient::from_env()? {
        None => run_watch().await,
        Some(source) => {
            let watch = WorkloadReconciler::connect_default().await?;
            let poll = WorkloadReconciler::connect_default().await?;
            let store_path = admission_store_path_from_env().map_err(|err| {
                ControllerError::new(format!("APPARATUS_ADMISSION_STORE_PATH: {err}"))
            })?;
            tokio::select! {
                result = watch.run_watch() => result,
                result = poll_ready_loop(source, poll, store_path) => result,
            }
        }
    }
}

async fn poll_ready_loop(
    source: HttpComponentsClient,
    reconciler: WorkloadReconciler,
    store_path: PathBuf,
) -> Result<(), ControllerError> {
    let interval = poll_interval_from_env();
    loop {
        match PersistentAdmissionStore::open(&store_path) {
            Ok(store) => {
                if let Err(err) = reconcile_ready(&source, &reconciler, &store).await {
                    eprintln!("apparatus-controller manifesto poll: {err}");
                }
            }
            Err(err) => eprintln!("apparatus-controller manifesto store: {err}"),
        }
        tokio::time::sleep(interval).await;
    }
}

fn poll_interval_from_env() -> Duration {
    let secs = std::env::var(MANIFESTO_POLL_INTERVAL_SECS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .unwrap_or(DEFAULT_POLL_SECS);
    Duration::from_secs(secs)
}
