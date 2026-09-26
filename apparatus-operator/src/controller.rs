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
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, Pod, PodSpec, Service, ServicePort, ServiceSpec,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
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
/// Label d'instance : Service selector = Pod (ADR-0605).
pub const INSTANCE_LABEL: &str = "app.kubernetes.io/instance";
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
    /// JSON VALID sans `envelopeReference`, ou `cosign verify` KO (fail-closed ADR-0008).
    SignatureUnverified,
}

impl fmt::Display for ScheduleRefuse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAdmissionRecord => f.write_str("missing AdmissionRecord"),
            Self::NotValid => f.write_str("AdmissionRecord is not VALID"),
            Self::MissingIsolation => f.write_str("missing isolation labels project+binding"),
            Self::MissingAdmittedCri => f.write_str("missing admitted cri pin"),
            Self::CriImageMismatch => f.write_str("CR criImage differs from admitted pin"),
            Self::SignatureUnverified => f.write_str("envelope signature missing or unverified"),
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
    /// Cible Cosign injectée (tests). Prod : [`AdmitTarget::from_schedule_env`].
    schedule_admit_target: Option<crate::admit::AdmitTarget>,
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
        Ok(Self {
            client,
            store_path,
            schedule_admit_target: None,
        })
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
        Ok(Self {
            client,
            store_path,
            schedule_admit_target: None,
        })
    }

    /// Injecte la cible Cosign/Transit pour la re-vérif schedule (tests IT).
    ///
    /// Prod lit l'env via [`crate::admit::AdmitTarget::from_schedule_env`]
    /// (`unsafe_code=forbid` interdit `set_var` dans les tests).
    #[must_use]
    pub fn with_schedule_admit_target(mut self, target: crate::admit::AdmitTarget) -> Self {
        self.schedule_admit_target = Some(target);
        self
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
        let labels = plugin_pod_labels(&isolation, &pod_name);
        let enroll_url = std::env::var("APPARATUS_PLUGIN_ENROLL_URL").ok();
        if let Ok(existing) = pods.get(&pod_name).await {
            if existing_plugin_pod_matches(
                &existing,
                &isolation,
                &digest,
                enroll_url.as_deref(),
                &pod_name,
            ) {
                self.ensure_plugin_service(&pod_name, labels).await?;
                return Ok(ReconcileOutcome::Scheduled { pod_name });
            }
            self.delete_plugin_pod(&pod_name).await?;
        }
        if let Err(refuse) =
            gate_schedule_signature(product.envelope_reference.as_deref(), self.admit_target())
        {
            eprintln!("apparatus-controller: SignatureUnverified digest={digest_str}");
            return Ok(ReconcileOutcome::Refused(refuse));
        }
        let mut spec = envelope_to_podspec(cri_image, &isolation)?;
        inject_plugin_workload_env(&mut spec, &isolation, &digest, enroll_url.as_deref());
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(pod_name.clone()),
                namespace: Some(PLUGINS_NAMESPACE.to_owned()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(spec),
            ..Pod::default()
        };
        match pods.create(&PostParams::default(), &pod).await {
            Ok(_) => {
                self.ensure_plugin_service(&pod_name, labels).await?;
                Ok(ReconcileOutcome::Scheduled { pod_name })
            }
            Err(err) if is_already_exists(&err) => {
                self.ensure_plugin_service(&pod_name, labels).await?;
                Ok(ReconcileOutcome::Scheduled { pod_name })
            }
            Err(err) => Err(ControllerError::new(format!("create Pod: {err}"))),
        }
    }

    fn admit_target(&self) -> Option<&crate::admit::AdmitTarget> {
        self.schedule_admit_target.as_ref()
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

    async fn ensure_plugin_service(
        &self,
        pod_name: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<(), ControllerError> {
        let services: Api<Service> = Api::namespaced(self.client.clone(), PLUGINS_NAMESPACE);
        let service = plugin_cluster_ip_service(pod_name, labels.clone());
        match services.create(&PostParams::default(), &service).await {
            Ok(_) => Ok(()),
            Err(err) if is_already_exists(&err) => match services.get(pod_name).await {
                Ok(existing) if service_selector_matches(&existing, &labels) => Ok(()),
                Ok(_) => {
                    self.replace_plugin_service_selector(&services, pod_name, &labels)
                        .await
                }
                Err(err) if is_not_found(&err) => services
                    .create(&PostParams::default(), &service)
                    .await
                    .map(|_| ())
                    .map_err(|err| ControllerError::new(format!("create Service: {err}"))),
                Err(err) => Err(ControllerError::new(format!("get Service: {err}"))),
            },
            Err(err) => Err(ControllerError::new(format!("create Service: {err}"))),
        }
    }

    async fn replace_plugin_service_selector(
        &self,
        services: &Api<Service>,
        pod_name: &str,
        labels: &BTreeMap<String, String>,
    ) -> Result<(), ControllerError> {
        services
            .patch(
                pod_name,
                &PatchParams::default(),
                &Patch::Merge(json!({ "spec": { "selector": labels } })),
            )
            .await
            .map(|_| ())
            .map_err(|err| ControllerError::new(format!("patch Service selector: {err}")))
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

fn plugin_pod_labels(isolation: &IsolationLabels, pod_name: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert(PROJECT_LABEL.to_owned(), isolation.project.clone());
    labels.insert(BINDING_LABEL.to_owned(), isolation.binding.clone());
    labels.insert(INSTANCE_LABEL.to_owned(), pod_name.to_owned());
    labels
}

fn service_selector_matches(service: &Service, labels: &BTreeMap<String, String>) -> bool {
    service
        .spec
        .as_ref()
        .and_then(|spec| spec.selector.as_ref())
        == Some(labels)
}

fn pod_label<'a>(pod: &'a Pod, key: &str) -> Option<&'a str> {
    pod.metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get(key))
        .map(String::as_str)
}

fn pod_env<'a>(pod: &'a Pod, name: &str) -> Option<&'a str> {
    pod.spec
        .as_ref()
        .and_then(|spec| spec.containers.first())
        .and_then(|container| container.env.as_ref())
        .and_then(|env| env.iter().find(|item| item.name == name))
        .and_then(|item| item.value.as_deref())
}

fn existing_plugin_pod_matches(
    pod: &Pod,
    isolation: &IsolationLabels,
    digest: &ReleaseDigest,
    enroll_url: Option<&str>,
    pod_name: &str,
) -> bool {
    pod_label(pod, PROJECT_LABEL) == Some(isolation.project.as_str())
        && pod_label(pod, BINDING_LABEL) == Some(isolation.binding.as_str())
        && pod_label(pod, INSTANCE_LABEL) == Some(pod_name)
        && pod_env(pod, "BINDING") == Some(isolation.binding.as_str())
        && pod_env(pod, "PROJECT_ID") == Some(isolation.project.as_str())
        && pod_env(pod, "RELEASE") == Some(digest.as_str())
        && match enroll_url.map(str::trim).filter(|value| !value.is_empty()) {
            None => true,
            Some(url) => pod_env(pod, "LAZARET_ENROLL_URL") == Some(url),
        }
}

/// Service ClusterIP du même nom que le Pod (ADR-0605).
#[must_use]
pub fn plugin_cluster_ip_service(pod_name: &str, labels: BTreeMap<String, String>) -> Service {
    Service {
        metadata: ObjectMeta {
            name: Some(pod_name.to_owned()),
            namespace: Some(PLUGINS_NAMESPACE.to_owned()),
            labels: Some(labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(ServiceSpec {
            type_: Some("ClusterIP".to_owned()),
            selector: Some(labels),
            ports: Some(vec![ServicePort {
                name: Some("http".to_owned()),
                port: PLUGIN_INVOKE_PORT,
                target_port: Some(IntOrString::Int(PLUGIN_INVOKE_PORT)),
                protocol: Some("TCP".to_owned()),
                ..ServicePort::default()
            }]),
            ..ServiceSpec::default()
        }),
        ..Service::default()
    }
}

fn inject_plugin_workload_env(
    spec: &mut PodSpec,
    isolation: &IsolationLabels,
    digest: &ReleaseDigest,
    enroll_url: Option<&str>,
) {
    let Some(container) = spec.containers.first_mut() else {
        return;
    };
    let mut env = container.env.take().unwrap_or_default();
    upsert_env(&mut env, "BINDING", isolation.binding.clone());
    upsert_env(&mut env, "PROJECT_ID", isolation.project.clone());
    upsert_env(&mut env, "RELEASE", digest.as_str().to_owned());
    // Enroll = HTTP 8080 (T14b). Session/invoke = HTTPS 8443. Pas de CA inventée ici.
    if let Some(url) = enroll_url.map(str::trim).filter(|value| !value.is_empty()) {
        upsert_env(&mut env, "LAZARET_ENROLL_URL", url.to_owned());
    }
    container.env = Some(env);
}

fn upsert_env(env: &mut Vec<EnvVar>, name: &str, value: String) {
    if let Some(existing) = env.iter_mut().find(|item| item.name == name) {
        existing.value = Some(value);
        return;
    }
    env.push(EnvVar {
        name: name.to_owned(),
        value: Some(value),
        ..EnvVar::default()
    });
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
    use crate::admit::{push_and_sign_envelope, AdmitTarget, Envelope};
    use crate::conformance::POLICY_ID;

    let store_path = admission_store_path_from_env()
        .map_err(|err| ControllerError::new(format!("APPARATUS_ADMISSION_STORE_PATH: {err}")))?;
    let cri_image = env_required("APPARATUS_CRI_IMAGE")?;
    let descriptor = ReleaseDigest::new(&env_required("APPARATUS_DESCRIPTOR_DIGEST")?)
        .map_err(|err| ControllerError::new(format!("APPARATUS_DESCRIPTOR_DIGEST: {err}")))?;
    let report = ReleaseDigest::new(&env_required("APPARATUS_REPORT_DIGEST")?)
        .map_err(|err| ControllerError::new(format!("APPARATUS_REPORT_DIGEST: {err}")))?;
    let target = AdmitTarget::from_schedule_env()
        .map_err(|err| ControllerError::new(format!("AdmitTarget: {err}")))?;
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
    store
        .bind_envelope(&record.descriptor_digest, &signed.reference)
        .map_err(|err| ControllerError::new(format!("bind_envelope: {err}")))?;
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

/// Gate Cosign avant `pods.create` : ref absente ou verify KO → refuse.
///
/// `injected` (tests) prime sur l'env. Env absentes + ref présente = refuse.
pub fn gate_schedule_signature(
    envelope_reference: Option<&str>,
    injected: Option<&crate::admit::AdmitTarget>,
) -> Result<(), ScheduleRefuse> {
    let Some(reference) = envelope_reference
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(ScheduleRefuse::SignatureUnverified);
    };
    match injected {
        Some(target) => crate::admit::verify_cosign_signature(target, reference)
            .map_err(|_| ScheduleRefuse::SignatureUnverified),
        None => {
            let target = crate::admit::AdmitTarget::from_schedule_env()
                .map_err(|_| ScheduleRefuse::SignatureUnverified)?;
            crate::admit::verify_cosign_signature(&target, reference)
                .map_err(|_| ScheduleRefuse::SignatureUnverified)
        }
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

#[cfg(test)]
mod plugin_service_tests {
    use super::{
        existing_plugin_pod_matches, inject_plugin_workload_env, plugin_cluster_ip_service,
        plugin_pod_labels, service_selector_matches, IsolationLabels, BINDING_LABEL,
        INSTANCE_LABEL, PLUGINS_NAMESPACE, PLUGIN_INVOKE_PORT, PROJECT_LABEL,
    };
    use crate::digest::ReleaseDigest;
    use k8s_openapi::api::core::v1::{Container, Pod, PodSpec, Service, ServiceSpec};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
    use std::collections::BTreeMap;

    fn gold_digest() -> ReleaseDigest {
        ReleaseDigest::new(
            "sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf",
        )
        .expect("digest")
    }

    fn matching_pod(
        isolation: &IsolationLabels,
        digest: &ReleaseDigest,
        pod_name: &str,
        enroll: Option<&str>,
    ) -> Pod {
        let mut spec = PodSpec {
            containers: vec![Container {
                name: "plugin".to_owned(),
                ..Container::default()
            }],
            ..PodSpec::default()
        };
        inject_plugin_workload_env(&mut spec, isolation, digest, enroll);
        Pod {
            metadata: ObjectMeta {
                name: Some(pod_name.to_owned()),
                labels: Some(plugin_pod_labels(isolation, pod_name)),
                ..ObjectMeta::default()
            },
            spec: Some(spec),
            ..Pod::default()
        }
    }

    #[test]
    fn cluster_ip_service_name_matches_pod_and_selector() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let pod_name = "plugin-98ba747fc572de29d76dfd08f9253778";
        let labels = plugin_pod_labels(&isolation, pod_name);
        let service = plugin_cluster_ip_service(pod_name, labels.clone());
        assert_eq!(service.metadata.name.as_deref(), Some(pod_name));
        assert_eq!(
            service.metadata.namespace.as_deref(),
            Some(PLUGINS_NAMESPACE)
        );
        let spec = service.spec.expect("spec");
        assert_eq!(spec.type_.as_deref(), Some("ClusterIP"));
        assert_eq!(spec.selector.as_ref(), Some(&labels));
        assert_eq!(
            labels.get(PROJECT_LABEL).map(String::as_str),
            Some("proj-1")
        );
        assert_eq!(
            labels.get(BINDING_LABEL).map(String::as_str),
            Some("bind-1")
        );
        assert_eq!(
            labels.get(INSTANCE_LABEL).map(String::as_str),
            Some(pod_name)
        );
        let port = spec.ports.expect("ports");
        assert_eq!(port[0].port, PLUGIN_INVOKE_PORT);
        assert_eq!(
            port[0].target_port,
            Some(IntOrString::Int(PLUGIN_INVOKE_PORT))
        );
    }

    #[test]
    fn injects_enroll_http_url_and_isolation_env() {
        let isolation = IsolationLabels::try_new(
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "11111111-2222-3333-4444-555555555555",
        )
        .expect("labels");
        let digest = ReleaseDigest::new(
            "sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf",
        )
        .expect("digest");
        let mut spec = PodSpec {
            containers: vec![Container {
                name: "plugin".to_owned(),
                ..Container::default()
            }],
            ..PodSpec::default()
        };
        let url = "http://oodhive-monolith.aiforall-gateway.svc.cluster.local:8080/lazaret/enroll";
        inject_plugin_workload_env(&mut spec, &isolation, &digest, Some(url));
        let env = spec.containers[0].env.as_ref().expect("env");
        let value = |name: &str| {
            env.iter()
                .find(|item| item.name == name)
                .and_then(|item| item.value.clone())
        };
        assert_eq!(
            value("BINDING").as_deref(),
            Some(isolation.binding.as_str())
        );
        assert_eq!(
            value("PROJECT_ID").as_deref(),
            Some(isolation.project.as_str())
        );
        assert_eq!(value("RELEASE").as_deref(), Some(digest.as_str()));
        assert_eq!(value("LAZARET_ENROLL_URL").as_deref(), Some(url));
    }

    #[test]
    fn skips_enroll_url_when_operator_env_absent() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let digest = ReleaseDigest::new(
            "sha256:98ba747fc572de29d76dfd08f92537782bf0353b652adcace10200020ee560cf",
        )
        .expect("digest");
        let mut spec = PodSpec {
            containers: vec![Container {
                name: "plugin".to_owned(),
                ..Container::default()
            }],
            ..PodSpec::default()
        };
        inject_plugin_workload_env(&mut spec, &isolation, &digest, Some("  "));
        let env = spec.containers[0].env.as_ref().expect("env");
        assert!(env.iter().all(|item| item.name != "LAZARET_ENROLL_URL"));
    }

    #[test]
    fn stale_service_selector_does_not_match_desired() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let pod_name = "plugin-98ba747fc572de29d76dfd08f9253778";
        let desired = plugin_pod_labels(&isolation, pod_name);
        let mut leftover = BTreeMap::new();
        leftover.insert(PROJECT_LABEL.to_owned(), "leftover".to_owned());
        let stale = Service {
            spec: Some(ServiceSpec {
                selector: Some(leftover),
                ..ServiceSpec::default()
            }),
            ..Service::default()
        };
        assert!(!service_selector_matches(&stale, &desired));
        assert!(service_selector_matches(
            &plugin_cluster_ip_service(pod_name, desired.clone()),
            &desired
        ));
    }

    #[test]
    fn keeps_pod_when_isolation_env_and_instance_match() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let digest = gold_digest();
        let pod_name = "plugin-98ba747fc572de29d76dfd08f9253778";
        let pod = matching_pod(&isolation, &digest, pod_name, None);
        assert!(existing_plugin_pod_matches(
            &pod, &isolation, &digest, None, pod_name
        ));
    }

    #[test]
    fn recreates_pod_on_binding_env_drift() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let other = IsolationLabels::try_new("proj-1", "bind-2").expect("other");
        let digest = gold_digest();
        let pod_name = "plugin-98ba747fc572de29d76dfd08f9253778";
        let pod = matching_pod(&other, &digest, pod_name, None);
        assert!(!existing_plugin_pod_matches(
            &pod, &isolation, &digest, None, pod_name
        ));
    }

    #[test]
    fn recreates_pod_when_enroll_url_set_but_missing() {
        let isolation = IsolationLabels::try_new("proj-1", "bind-1").expect("labels");
        let digest = gold_digest();
        let pod_name = "plugin-98ba747fc572de29d76dfd08f9253778";
        let pod = matching_pod(&isolation, &digest, pod_name, None);
        assert!(!existing_plugin_pod_matches(
            &pod,
            &isolation,
            &digest,
            Some("http://oodhive-monolith.aiforall-gateway.svc.cluster.local:8080/lazaret/enroll"),
            pod_name
        ));
    }
}

#[cfg(test)]
mod schedule_signature_gate_tests {
    use super::{gate_schedule_signature, ScheduleRefuse};

    #[test]
    fn missing_envelope_reference_refuses_without_kind() {
        assert_eq!(
            gate_schedule_signature(None, None),
            Err(ScheduleRefuse::SignatureUnverified)
        );
        assert_eq!(
            gate_schedule_signature(Some(""), None),
            Err(ScheduleRefuse::SignatureUnverified)
        );
        assert_eq!(
            gate_schedule_signature(Some("   "), None),
            Err(ScheduleRefuse::SignatureUnverified)
        );
    }

    #[test]
    fn present_ref_without_admit_target_env_refuses() {
        // Env schedule absentes (unsafe_code=forbid : pas de set_var) ⇒ fail-closed.
        assert_eq!(
            gate_schedule_signature(Some("127.0.0.1:5000/repo@sha256:deadbeef"), None),
            Err(ScheduleRefuse::SignatureUnverified)
        );
    }
}
