//! Contrôleur P4 (feature `controller`) : schedule si le JSON produit est VALID.
//!
//! Porte M3 = [`crate::admission::PersistentAdmissionStore`]. La CR Kind n'est
//! pas le store produit : le pin CRI vient du JSON (`bind_cri`) ; l'isolation
//! reste sur les labels CR **après** le gate JSON. Pas de listener HTTP.
//! Isolation manquante = refus fermé. `spec.image` = pin CRI `@sha256`, jamais
//! `latest`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Container, ContainerPort, Pod, PodSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Api, DeleteParams, DynamicObject, Patch, PatchParams, PostParams};
use kube::core::{ApiResource, GroupVersionKind, ResourceExt};
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Client, Config};
use serde_json::{json, Value};

use crate::admission::{
    admission_store_path_from_env, validate_cri_pin as validate_admitted_cri_pin, AdmissionRecord,
    AdmissionStore, PersistentAdmissionStore,
};
use crate::digest::ReleaseDigest;

/// Namespace des identités operator / CR d'admission.
pub const SYSTEM_NAMESPACE: &str = "apparatus-system";
/// Namespace des workloads plugin.
pub const PLUGINS_NAMESPACE: &str = "apparatus-plugins";
/// Port HTTP `INVOKE_PATH` exposé sur le Pod plugin.
pub const PLUGIN_INVOKE_PORT: i32 = 8080;
/// Groupe API CRD.
pub const GROUP: &str = "apparatus.aiforall.dev";
/// Version CRD.
pub const VERSION: &str = "v1alpha1";
/// Kind CRD.
pub const KIND: &str = "AdmissionRecord";
/// Label projet (isolation).
pub const PROJECT_LABEL: &str = "apparatus.aiforall.dev/project";
/// Label binding (isolation).
pub const BINDING_LABEL: &str = "apparatus.aiforall.dev/binding";
/// Phase cluster écrite uniquement par admit.
pub const VALID_PHASE: &str = "VALID";

/// Labels d'isolation par projet et par binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationLabels {
    /// Identifiant projet (ex. `presses-nord`).
    pub project: String,
    /// Identifiant binding (ex. `shift-0300`).
    pub binding: String,
}

impl IsolationLabels {
    /// Construit les labels si les deux valeurs sont non vides.
    ///
    /// # Errors
    ///
    /// Projet ou binding vide → isolation manquante.
    pub fn try_new(
        project: impl Into<String>,
        binding: impl Into<String>,
    ) -> Result<Self, ControllerError> {
        let project = project.into();
        let binding = binding.into();
        if project.is_empty() || binding.is_empty() {
            return Err(ControllerError::new(
                "missing isolation labels project+binding",
            ));
        }
        Ok(Self { project, binding })
    }
}

/// Refus de schedule (pas de Pod créé).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleRefuse {
    /// Pas de ligne JSON VALID, et pas de CR — ou CR VALID sans ligne JSON (CR-only).
    MissingAdmissionRecord,
    /// Pas de ligne JSON VALID et CR présente avec phase ≠ [`VALID_PHASE`].
    NotValid,
    /// JSON VALID mais labels projet+binding absents sur la CR.
    MissingIsolation,
    /// JSON VALID sans pin CRI persisté (`bind_cri`).
    MissingAdmittedCri,
    /// `spec.criImage` CR présent et différent du pin JSON.
    CriImageMismatch,
}

impl fmt::Display for ScheduleRefuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAdmissionRecord => f.write_str("missing AdmissionRecord"),
            Self::NotValid => f.write_str("AdmissionRecord is not VALID"),
            Self::MissingIsolation => f.write_str("missing isolation labels project+binding"),
            Self::MissingAdmittedCri => f.write_str("missing admitted cri pin"),
            Self::CriImageMismatch => f.write_str("CR criImage differs from admitted pin"),
        }
    }
}

/// Résultat d'une réconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// Pod créé ou déjà présent.
    Scheduled {
        /// Nom du Pod dans [`PLUGINS_NAMESPACE`].
        pod_name: String,
    },
    /// Aucun Pod (refus fermé).
    Refused(ScheduleRefuse),
}

/// Erreur contrôleur / client Kubernetes.
#[derive(Debug)]
pub struct ControllerError {
    message: String,
}

impl ControllerError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ControllerError {}

/// Client de réconciliation (watch CR → Pod si JSON VALID).
pub struct WorkloadReconciler {
    client: Client,
    store_path: PathBuf,
}

impl WorkloadReconciler {
    /// Construit un client depuis un kubeconfig Kind/exporté et le JSON produit.
    ///
    /// # Errors
    ///
    /// Kubeconfig illisible ou API injoignable.
    pub async fn connect(kubeconfig: &Path, store_path: &Path) -> Result<Self, ControllerError> {
        let store_path = store_path.to_path_buf();
        let raw = kube::config::Kubeconfig::read_from(kubeconfig).map_err(|err| {
            ControllerError::new(format!("read kubeconfig {}: {err}", kubeconfig.display()))
        })?;
        let cfg = Config::from_custom_kubeconfig(raw, &kube::config::KubeConfigOptions::default())
            .await
            .map_err(|err| ControllerError::new(format!("kube config: {err}")))?;
        let client = Client::try_from(cfg)
            .map_err(|err| ControllerError::new(format!("kube client: {err}")))?;
        Ok(Self { client, store_path })
    }

    /// Client inferé (`KUBECONFIG` / in-cluster) + [`admission_store_path_from_env`].
    ///
    /// # Errors
    ///
    /// Configuration Kubernetes absente, ou `APPARATUS_ADMISSION_STORE_PATH` unset/vide.
    pub async fn connect_default() -> Result<Self, ControllerError> {
        let store_path = admission_store_path_from_env().map_err(|err| {
            ControllerError::new(format!("APPARATUS_ADMISSION_STORE_PATH: {err}"))
        })?;
        let client = Client::try_default()
            .await
            .map_err(|err| ControllerError::new(format!("kube client: {err}")))?;
        Ok(Self { client, store_path })
    }

    /// Écrit la CR `AdmissionRecord` et le statut [`VALID_PHASE`] (chemin admit).
    ///
    /// # Errors
    ///
    /// Échec API create/replace/patch status.
    pub async fn apply_valid_record(
        &self,
        record: &AdmissionRecord,
        cri_image: &str,
        isolation: Option<&IsolationLabels>,
    ) -> Result<(), ControllerError> {
        validate_cri_pin(cri_image)?;
        let name = cr_name_for(&record.descriptor_digest);
        let api = admission_api(self.client.clone());
        let _ = api.delete(&name, &DeleteParams::default()).await;
        let mut labels = serde_json::Map::new();
        if let Some(iso) = isolation {
            labels.insert(PROJECT_LABEL.to_owned(), json!(iso.project));
            labels.insert(BINDING_LABEL.to_owned(), json!(iso.binding));
        }
        let body = json!({
            "apiVersion": format!("{GROUP}/{VERSION}"),
            "kind": KIND,
            "metadata": {
                "name": name,
                "namespace": SYSTEM_NAMESPACE,
                "labels": Value::Object(labels),
            },
            "spec": {
                "descriptorDigest": record.descriptor_digest.as_str(),
                "policyVersion": record.policy_version,
                "reportDigest": record.report_digest.as_str(),
                "criImage": cri_image,
            }
        });
        let obj: DynamicObject = serde_json::from_value(body)
            .map_err(|err| ControllerError::new(format!("serialize AdmissionRecord CR: {err}")))?;
        api.create(&PostParams::default(), &obj)
            .await
            .map_err(|err| ControllerError::new(format!("create AdmissionRecord: {err}")))?;
        let status_patch = json!({
            "apiVersion": format!("{GROUP}/{VERSION}"),
            "kind": KIND,
            "metadata": { "name": name, "namespace": SYSTEM_NAMESPACE },
            "status": { "phase": VALID_PHASE }
        });
        api.patch_status(&name, &PatchParams::default(), &Patch::Merge(status_patch))
            .await
            .map_err(|err| ControllerError::new(format!("patch AdmissionRecord status: {err}")))?;
        Ok(())
    }

    /// Schedule un Pod seulement si le JSON produit a une ligne VALID pour ce digest.
    ///
    /// # Errors
    ///
    /// Échec API get/create Pod (un refus métier est [`ReconcileOutcome::Refused`]).
    pub async fn reconcile_digest(
        &self,
        digest: &ReleaseDigest,
    ) -> Result<ReconcileOutcome, ControllerError> {
        let api = admission_api(self.client.clone());
        let name = cr_name_for(digest);
        let obj = match api.get(&name).await {
            Ok(obj) => obj,
            Err(err) if is_not_found(&err) => {
                return Ok(ReconcileOutcome::Refused(
                    ScheduleRefuse::MissingAdmissionRecord,
                ));
            }
            Err(err) => {
                return Err(ControllerError::new(format!("get AdmissionRecord: {err}")));
            }
        };
        self.reconcile_object(&obj).await
    }

    fn product_line(
        &self,
        digest: &ReleaseDigest,
    ) -> Result<Option<AdmissionRecord>, ControllerError> {
        let store = PersistentAdmissionStore::open(&self.store_path).map_err(|err| {
            ControllerError::new(format!(
                "admission store {}: {err}",
                self.store_path.display()
            ))
        })?;
        Ok(store.get(digest))
    }

    async fn reconcile_object(
        &self,
        obj: &DynamicObject,
    ) -> Result<ReconcileOutcome, ControllerError> {
        let spec = obj.data.get("spec").and_then(Value::as_object);
        let Some(digest_str) = spec
            .and_then(|spec| spec.get("descriptorDigest"))
            .and_then(Value::as_str)
        else {
            return Err(ControllerError::new(
                "AdmissionRecord spec.descriptorDigest missing",
            ));
        };
        let digest = ReleaseDigest::new(digest_str)
            .map_err(|err| ControllerError::new(format!("descriptor digest: {err}")))?;
        let Some(product) = self.product_line(&digest)? else {
            let phase = obj
                .data
                .get("status")
                .and_then(Value::as_object)
                .and_then(|status| status.get("phase"))
                .and_then(Value::as_str);
            let refuse = if phase == Some(VALID_PHASE) {
                ScheduleRefuse::MissingAdmissionRecord
            } else {
                ScheduleRefuse::NotValid
            };
            return Ok(ReconcileOutcome::Refused(refuse));
        };
        let Some(cri_image) = product.cri_image.as_deref() else {
            return Ok(ReconcileOutcome::Refused(
                ScheduleRefuse::MissingAdmittedCri,
            ));
        };
        if let Some(cr_cri) = spec
            .and_then(|spec| spec.get("criImage"))
            .and_then(Value::as_str)
        {
            if cr_cri != cri_image {
                return Ok(ReconcileOutcome::Refused(ScheduleRefuse::CriImageMismatch));
            }
        }
        let isolation = match isolation_from_labels(obj.labels()) {
            Ok(iso) => iso,
            Err(refuse) => return Ok(ReconcileOutcome::Refused(refuse)),
        };
        let pod_name = pod_name_for(&digest);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), PLUGINS_NAMESPACE);
        if pods.get(&pod_name).await.is_ok() {
            return Ok(ReconcileOutcome::Scheduled { pod_name });
        }
        let spec = envelope_to_podspec(cri_image, &isolation)?;
        let mut labels = BTreeMap::new();
        labels.insert(PROJECT_LABEL.to_owned(), isolation.project.clone());
        labels.insert(BINDING_LABEL.to_owned(), isolation.binding.clone());
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(pod_name.clone()),
                namespace: Some(PLUGINS_NAMESPACE.to_owned()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            spec: Some(spec),
            ..Pod::default()
        };
        match pods.create(&PostParams::default(), &pod).await {
            Ok(_) => Ok(ReconcileOutcome::Scheduled { pod_name }),
            Err(err) if is_already_exists(&err) => Ok(ReconcileOutcome::Scheduled { pod_name }),
            Err(err) => Err(ControllerError::new(format!("create Pod: {err}"))),
        }
    }

    /// Attend que le Pod soit `Running` et retourne `spec.containers[0].image`.
    ///
    /// # Errors
    ///
    /// Timeout, phase Failed, ou image absente.
    pub async fn wait_pod_running(
        &self,
        pod_name: &str,
        timeout: Duration,
    ) -> Result<String, ControllerError> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), PLUGINS_NAMESPACE);
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            match pods.get(pod_name).await {
                Ok(pod) => {
                    let phase = pod
                        .status
                        .as_ref()
                        .and_then(|status| status.phase.as_deref())
                        .unwrap_or("");
                    if phase == "Running" {
                        return pod_image(&pod);
                    }
                    if phase == "Failed" {
                        return Err(ControllerError::new(format!(
                            "pod {pod_name} Failed ({})",
                            waiting_reason(&pod)
                        )));
                    }
                    let waiting = waiting_reason(&pod);
                    if waiting.contains("ErrImage")
                        || waiting.contains("ImagePullBackOff")
                        || waiting.contains("InvalidImageName")
                    {
                        return Err(ControllerError::new(format!(
                            "pod {pod_name} image error: {waiting}"
                        )));
                    }
                }
                Err(err) if is_not_found(&err) => {}
                Err(err) => {
                    return Err(ControllerError::new(format!("get pod {pod_name}: {err}")));
                }
            }
            if tokio::time::Instant::now() > deadline {
                return Err(ControllerError::new(format!(
                    "timeout waiting for pod {pod_name} Running"
                )));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Indique si le Pod existe dans [`PLUGINS_NAMESPACE`].
    ///
    /// # Errors
    ///
    /// Échec API autre que 404.
    pub async fn plugin_pod_exists(&self, pod_name: &str) -> Result<bool, ControllerError> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), PLUGINS_NAMESPACE);
        match pods.get(pod_name).await {
            Ok(_) => Ok(true),
            Err(err) if is_not_found(&err) => Ok(false),
            Err(err) => Err(ControllerError::new(format!("get pod: {err}"))),
        }
    }

    /// Supprime le Pod plugin (404 ignoré) et attend qu'il soit Gone.
    ///
    /// # Errors
    ///
    /// Échec API delete, ou timeout Terminating.
    pub async fn delete_plugin_pod(&self, pod_name: &str) -> Result<(), ControllerError> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), PLUGINS_NAMESPACE);
        let params = DeleteParams {
            grace_period_seconds: Some(0),
            ..DeleteParams::default()
        };
        match pods.delete(pod_name, &params).await {
            Ok(_) => {}
            Err(err) if is_not_found(&err) => return Ok(()),
            Err(err) => return Err(ControllerError::new(format!("delete pod: {err}"))),
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
        loop {
            match self.plugin_pod_exists(pod_name).await {
                Ok(false) => return Ok(()),
                Ok(true) => {}
                Err(err) => return Err(err),
            }
            if tokio::time::Instant::now() > deadline {
                return Err(ControllerError::new(format!(
                    "timeout waiting for pod {pod_name} Gone"
                )));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// Supprime la CR (404 ignoré).
    ///
    /// # Errors
    ///
    /// Échec API delete.
    pub async fn delete_admission_record(
        &self,
        digest: &ReleaseDigest,
    ) -> Result<(), ControllerError> {
        let api = admission_api(self.client.clone());
        let name = cr_name_for(digest);
        match api.delete(&name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(err) if is_not_found(&err) => Ok(()),
            Err(err) => Err(ControllerError::new(format!(
                "delete AdmissionRecord: {err}"
            ))),
        }
    }

    /// Watch les CR et réconcilie (binaire `apparatus-controller`).
    ///
    /// # Errors
    ///
    /// Échec du watch Kubernetes.
    pub async fn run_watch(self) -> Result<(), ControllerError> {
        let api = admission_api(self.client.clone());
        let mut stream = watcher(api, watcher::Config::default())
            .default_backoff()
            .applied_objects()
            .boxed();
        while let Some(item) = stream.next().await {
            match item {
                Ok(obj) => {
                    if let Err(err) = self.reconcile_object(&obj).await {
                        eprintln!("apparatus-controller reconcile: {err}");
                    }
                }
                Err(err) => eprintln!("apparatus-controller watch: {err}"),
            }
        }
        Ok(())
    }
}

/// Convertit l'image CRI pinée en `PodSpec` (jamais `latest`).
///
/// # Errors
///
/// Isolation vide, image non pinée `@sha256`, ou tag `latest`.
pub fn envelope_to_podspec(
    cri_image: &str,
    isolation: &IsolationLabels,
) -> Result<PodSpec, ControllerError> {
    if isolation.project.is_empty() || isolation.binding.is_empty() {
        return Err(ControllerError::new(
            "missing isolation labels project+binding",
        ));
    }
    validate_cri_pin(cri_image)?;
    Ok(PodSpec {
        containers: vec![Container {
            name: "plugin".to_owned(),
            image: Some(cri_image.to_owned()),
            image_pull_policy: Some("IfNotPresent".to_owned()),
            ports: Some(vec![ContainerPort {
                name: Some("invoke".to_owned()),
                container_port: PLUGIN_INVOKE_PORT,
                protocol: Some("TCP".to_owned()),
                ..ContainerPort::default()
            }]),
            ..Container::default()
        }],
        restart_policy: Some("Always".to_owned()),
        automount_service_account_token: Some(false),
        ..PodSpec::default()
    })
}

/// Nom DNS-1123 de la CR pour un digest 0002.
#[must_use]
pub fn cr_name_for(digest: &ReleaseDigest) -> String {
    format!("sha256-{}", digest.hex_part())
}

/// Nom DNS-1123 du Pod plugin.
#[must_use]
pub fn pod_name_for(digest: &ReleaseDigest) -> String {
    let hex = digest.hex_part();
    let take = hex.len().min(32);
    format!("plugin-{}", &hex[..take])
}

/// Watch infini (binaire controller, feature `controller`).
///
/// # Errors
///
/// Voir [`WorkloadReconciler::run_watch`].
pub async fn run_watch() -> Result<(), ControllerError> {
    WorkloadReconciler::connect_default()
        .await?
        .run_watch()
        .await
}

/// Signe l'enveloppe T6, persiste `VALID` dans le JSON produit, puis applique
/// la CR Kind (binaire admit, features `admit`+`controller`).
///
/// Store produit : [`crate::admission::PersistentAdmissionStore`] (env
/// [`crate::admission::ADMISSION_STORE_PATH_ENV`] obligatoire, TCB, volume
/// exclusif `apparatus-admit`). Sans env : erreur, pas d'admit ni de CR.
/// La CR reste l'apply optionnel T10.
///
/// # Errors
///
/// Variables d'environnement manquantes, ouverture du JSON, signature, ou apply CR.
#[cfg(feature = "admit")]
pub async fn sign_envelope_and_apply_from_env() -> Result<(), ControllerError> {
    use crate::admission::{
        admission_store_path_from_env, AdmissionStore, AdmitInput, PersistentAdmissionStore,
    };
    use crate::admit::{push_and_sign_envelope, AdmitTarget, Envelope, RegistryAuth};
    use crate::conformance::POLICY_ID;

    let store_path = admission_store_path_from_env()
        .map_err(|err| ControllerError::new(format!("APPARATUS_ADMISSION_STORE_PATH: {err}")))?;
    let cri_image = env_required("APPARATUS_CRI_IMAGE")?;
    let descriptor = ReleaseDigest::new(&env_required("APPARATUS_DESCRIPTOR_DIGEST")?)
        .map_err(|err| ControllerError::new(format!("APPARATUS_DESCRIPTOR_DIGEST: {err}")))?;
    let report = ReleaseDigest::new(&env_required("APPARATUS_REPORT_DIGEST")?)
        .map_err(|err| ControllerError::new(format!("APPARATUS_REPORT_DIGEST: {err}")))?;
    let target = AdmitTarget {
        registry_host: env_required("APPARATUS_REGISTRY_HOST")?,
        registry_network: env_required("APPARATUS_REGISTRY_NETWORK")?,
        docker_network: env_required("APPARATUS_DOCKER_NETWORK")?,
        vault_addr_host: env_required("APPARATUS_VAULT_ADDR")?,
        vault_addr_network: env_required("APPARATUS_VAULT_ADDR_NETWORK")?,
        vault_token: env_required("APPARATUS_VAULT_TOKEN")?,
        transit_key: env_required("APPARATUS_TRANSIT_KEY")?,
        repository: env_or("APPARATUS_REPOSITORY", "apparatus/envelope"),
        tag: env_or("APPARATUS_ENVELOPE_TAG", "t10"),
        auth: RegistryAuth {
            username: env_required("APPARATUS_REGISTRY_USER")?,
            password: env_required("APPARATUS_REGISTRY_PASSWORD")?,
        },
    };
    let envelope = Envelope {
        descriptor_digest: descriptor.clone(),
        cri_image: cri_image.clone(),
    };
    let signed = push_and_sign_envelope(&target, &envelope)
        .await
        .map_err(|err| ControllerError::new(format!("push_and_sign_envelope: {err}")))?;
    let mut store = PersistentAdmissionStore::open(&store_path).map_err(|err| {
        ControllerError::new(format!("admission store {}: {err}", store_path.display()))
    })?;
    let record = store
        .admit(AdmitInput {
            descriptor_digest: descriptor,
            observed_descriptor: signed.catalog_digest.clone(),
            policy_id: POLICY_ID.to_owned(),
            report_digest: report,
            conformance_passed: true,
            signature_verified: signed.signature_verified,
            claimed_verified: false,
        })
        .map_err(|err| ControllerError::new(format!("admit: {err}")))?;
    store
        .bind_cri(&record.descriptor_digest, &signed.cri_image)
        .map_err(|err| ControllerError::new(format!("bind_cri: {err}")))?;
    let isolation = match (
        std::env::var("APPARATUS_PROJECT").ok(),
        std::env::var("APPARATUS_BINDING").ok(),
    ) {
        (Some(project), Some(binding)) => Some(IsolationLabels::try_new(project, binding)?),
        _ => None,
    };
    let reconciler = WorkloadReconciler::connect_default().await?;
    reconciler
        .apply_valid_record(&record, &signed.cri_image, isolation.as_ref())
        .await
}

fn admission_api(client: Client) -> Api<DynamicObject> {
    let gvk = GroupVersionKind::gvk(GROUP, VERSION, KIND);
    let ar = ApiResource::from_gvk(&gvk);
    Api::namespaced_with(client, SYSTEM_NAMESPACE, &ar)
}

fn isolation_from_labels(
    labels: &BTreeMap<String, String>,
) -> Result<IsolationLabels, ScheduleRefuse> {
    let project = labels
        .get(PROJECT_LABEL)
        .map(String::as_str)
        .filter(|value| !value.is_empty());
    let binding = labels
        .get(BINDING_LABEL)
        .map(String::as_str)
        .filter(|value| !value.is_empty());
    match (project, binding) {
        (Some(project), Some(binding)) => Ok(IsolationLabels {
            project: project.to_owned(),
            binding: binding.to_owned(),
        }),
        _ => Err(ScheduleRefuse::MissingIsolation),
    }
}

fn validate_cri_pin(cri_image: &str) -> Result<(), ControllerError> {
    match validate_admitted_cri_pin(cri_image) {
        Ok(()) => Ok(()),
        Err(msg) if msg.contains("latest") => {
            Err(ControllerError::new("spec.image must not use tag latest"))
        }
        Err(_) => Err(ControllerError::new(
            "spec.image must be pinned as name@sha256:<digest>",
        )),
    }
}

fn pod_image(pod: &Pod) -> Result<String, ControllerError> {
    pod.spec
        .as_ref()
        .and_then(|spec| spec.containers.first())
        .and_then(|container| container.image.clone())
        .ok_or_else(|| ControllerError::new("pod spec.containers[0].image missing"))
}

fn waiting_reason(pod: &Pod) -> String {
    pod.status
        .as_ref()
        .and_then(|status| status.container_statuses.as_ref())
        .and_then(|statuses| statuses.first())
        .and_then(|status| status.state.as_ref())
        .and_then(|state| state.waiting.as_ref())
        .map(|waiting| {
            format!(
                "{}: {}",
                waiting.reason.as_deref().unwrap_or("Waiting"),
                waiting.message.as_deref().unwrap_or("")
            )
        })
        .unwrap_or_default()
}

fn is_not_found(err: &kube::Error) -> bool {
    matches!(err, kube::Error::Api(status) if status.code == 404)
}

fn is_already_exists(err: &kube::Error) -> bool {
    matches!(err, kube::Error::Api(status) if status.code == 409)
}

#[cfg(feature = "admit")]
fn env_required(name: &str) -> Result<String, ControllerError> {
    std::env::var(name)
        .map_err(|_| ControllerError::new(format!("missing required environment variable {name}")))
}

#[cfg(feature = "admit")]
fn env_or(name: &str, fallback: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| fallback.to_owned())
}
