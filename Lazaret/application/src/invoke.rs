//! Server invoke bound to the current binding (P0 DTOs, live Manifesto grants).

use std::sync::Arc;
use std::time::Duration;

use apparatus_contracts::{Capability, InvokeRequest, InvokeResponse, INVOKE_PATH};
use async_trait::async_trait;
use lazaret_domain::{
    AsyncKvStore, CallOrigin, ConnectorError, ConnectorProxy, GrantAuthorizationRequest,
    GrantDecision, GrantDenyReason, GrantFetchError, IdentityError, SecretError, SecretResolver,
};
use thiserror::Error;
use uuid::Uuid;

use crate::{GrantService, IdentityService};

/// Resolves an isolated plugin HTTP base URL by binding (and instance).
///
/// `None` keeps in-process dispatch (T7). Implementations must not call kube.
#[async_trait]
pub trait PluginEndpointLocator: Send + Sync {
    /// Return `http://host:port` for this binding, or `None` to stay in-process.
    async fn locate(&self, binding_id: &str, instance_id: &str) -> Option<String>;
}

/// Default locator: no hop.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyPluginLocator;

#[async_trait]
impl PluginEndpointLocator for EmptyPluginLocator {
    async fn locate(&self, _binding_id: &str, _instance_id: &str) -> Option<String> {
        None
    }
}

/// Test/config locator: one injected URL for every binding.
#[derive(Debug, Clone)]
pub struct StaticPluginLocator {
    url: String,
}

impl StaticPluginLocator {
    /// Wire a plugin base URL (`http://127.0.0.1:port` from the test).
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[async_trait]
impl PluginEndpointLocator for StaticPluginLocator {
    async fn locate(&self, _binding_id: &str, _instance_id: &str) -> Option<String> {
        if self.url.trim().is_empty() {
            None
        } else {
            Some(self.url.clone())
        }
    }
}

/// Errors from [`InvokeService`].
#[derive(Debug, Error)]
pub enum InvokeError {
    /// Missing/invalid Lazaret session.
    #[error("unauthorized")]
    Unauthorized,
    /// Grant intersection denied.
    #[error("forbidden: {0}")]
    Forbidden(GrantDenyReason),
    /// Malformed request.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// Manifesto consult failed.
    #[error("grant consult failed")]
    ConsultFailed,
    /// Payload exceeded the platform bound.
    #[error("payload too large")]
    PayloadTooLarge,
    /// Platform I/O failed closed.
    #[error("operation failed")]
    Failed,
}

impl From<IdentityError> for InvokeError {
    fn from(_: IdentityError) -> Self {
        Self::Unauthorized
    }
}

impl From<GrantFetchError> for InvokeError {
    fn from(value: GrantFetchError) -> Self {
        match value {
            GrantFetchError::NotFound => Self::Forbidden(GrantDenyReason::ProjectMismatch),
            GrantFetchError::Transport(_) => Self::ConsultFailed,
        }
    }
}

impl From<ConnectorError> for InvokeError {
    fn from(value: ConnectorError) -> Self {
        match value {
            ConnectorError::Rejected | ConnectorError::Unknown => {
                Self::BadRequest("connector rejected".to_owned())
            }
            ConnectorError::PayloadTooLarge => Self::PayloadTooLarge,
            ConnectorError::FetchFailed => Self::Failed,
        }
    }
}

impl From<SecretError> for InvokeError {
    fn from(value: SecretError) -> Self {
        match value {
            SecretError::InvalidReference => {
                Self::BadRequest("invalid secret reference".to_owned())
            }
            SecretError::ResolveFailed => Self::Failed,
        }
    }
}

/// HTTP invoke application service.
pub struct InvokeService {
    identity: Arc<IdentityService>,
    grants: Arc<GrantService>,
    kv: Arc<dyn AsyncKvStore>,
    secrets: Arc<dyn SecretResolver>,
    connectors: Arc<dyn ConnectorProxy>,
    locator: Arc<dyn PluginEndpointLocator>,
    http: reqwest::Client,
}

impl InvokeService {
    /// Wire collaborators with an empty locator (in-process dispatch).
    #[must_use]
    pub fn new(
        identity: Arc<IdentityService>,
        grants: Arc<GrantService>,
        kv: Arc<dyn AsyncKvStore>,
        secrets: Arc<dyn SecretResolver>,
        connectors: Arc<dyn ConnectorProxy>,
    ) -> Self {
        Self::new_with_locator(
            identity,
            grants,
            kv,
            secrets,
            connectors,
            Arc::new(EmptyPluginLocator),
        )
    }

    /// Wire collaborators once, including the optional plugin hop locator.
    #[must_use]
    pub fn new_with_locator(
        identity: Arc<IdentityService>,
        grants: Arc<GrantService>,
        kv: Arc<dyn AsyncKvStore>,
        secrets: Arc<dyn SecretResolver>,
        connectors: Arc<dyn ConnectorProxy>,
        locator: Arc<dyn PluginEndpointLocator>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            identity,
            grants,
            kv,
            secrets,
            connectors,
            locator,
            http,
        }
    }

    /// Verify session, live grant, then dispatch P0 operations.
    ///
    /// HTTP invoke is always [`CallOrigin::Background`]. End-user `principal`
    /// is not accepted on this surface.
    ///
    /// # Errors
    ///
    /// Returns [`InvokeError`] on authz, validation, or platform failure.
    pub async fn invoke(
        &self,
        session_token: &str,
        project_id: Uuid,
        request: InvokeRequest,
    ) -> Result<InvokeResponse, InvokeError> {
        request.validate().map_err(map_contract_error)?;
        let proof = self.identity.verify_session(session_token)?;
        if request.binding_id.as_str() != proof.identity.binding.to_string() {
            return Err(InvokeError::Forbidden(GrantDenyReason::ProjectMismatch));
        }
        let capability = capability_for_operation(&request.operation)?;
        let decision = self
            .grants
            .authorize(&GrantAuthorizationRequest {
                identity: proof.identity.clone(),
                capability: capability.as_str().to_owned(),
                declared: Vec::new(),
                origin: CallOrigin::Background,
                project_id,
            })
            .await?;
        match decision {
            GrantDecision::Allow => {}
            GrantDecision::Deny { reason } => return Err(InvokeError::Forbidden(reason)),
        }
        let instance = proof.identity.instance.to_string();
        if let Some(endpoint) = self
            .locator
            .locate(request.binding_id.as_str(), &instance)
            .await
        {
            return self.forward_invoke(&endpoint, request).await;
        }
        let result = self.dispatch(&request).await?;
        let response = InvokeResponse {
            binding_id: request.binding_id.clone(),
            operation_id: request.operation_id,
            result,
        };
        response.validate().map_err(map_contract_error)?;
        Ok(response)
    }

    async fn forward_invoke(
        &self,
        endpoint: &str,
        request: InvokeRequest,
    ) -> Result<InvokeResponse, InvokeError> {
        let url = plugin_invoke_url(endpoint);
        let response = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|_| InvokeError::Failed)?;
        if !response.status().is_success() {
            return Err(InvokeError::Failed);
        }
        let body: InvokeResponse = response.json().await.map_err(|_| InvokeError::Failed)?;
        body.validate().map_err(map_contract_error)?;
        Ok(body)
    }

    async fn dispatch(&self, request: &InvokeRequest) -> Result<serde_json::Value, InvokeError> {
        match request.operation.as_str() {
            "kv.get" => {
                let key = param_str(&request.params, "key")?;
                reject_urlish(&request.params)?;
                let found = self
                    .kv
                    .get(&request.binding_id, key)
                    .await
                    .map_err(|_| InvokeError::Failed)?;
                Ok(found.map_or_else(
                    || serde_json::json!({"found": false}),
                    |bytes| {
                        let text = String::from_utf8_lossy(&bytes).into_owned();
                        serde_json::json!({"found": true, "value": text})
                    },
                ))
            }
            "kv.put" => {
                let key = param_str(&request.params, "key")?;
                let value = param_str(&request.params, "value")?;
                reject_urlish(&request.params)?;
                let cas = request
                    .params
                    .get("cas")
                    .and_then(serde_json::Value::as_i64);
                let version = self
                    .kv
                    .put(&request.binding_id, key, value.as_bytes(), cas)
                    .await
                    .map_err(map_kv_put_error)?;
                Ok(serde_json::json!({"stored": true, "cas_version": version}))
            }
            "kv.delete" => {
                let key = param_str(&request.params, "key")?;
                reject_urlish(&request.params)?;
                let removed = self
                    .kv
                    .delete(&request.binding_id, key)
                    .await
                    .map_err(|_| InvokeError::Failed)?;
                Ok(serde_json::json!({"removed": removed}))
            }
            "connector.fetch" => {
                reject_urlish(&request.params)?;
                let name = param_str(&request.params, "connector")?;
                let path = request
                    .params
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("/");
                let injected = if let Some(reference) = request
                    .params
                    .get("secret")
                    .and_then(serde_json::Value::as_str)
                {
                    Some(self.secrets.resolve(reference).await?)
                } else {
                    None
                };
                let body = self
                    .connectors
                    .fetch(name, path, injected.as_deref())
                    .await?;
                let text = String::from_utf8_lossy(&body).into_owned();
                Ok(serde_json::json!({"body": text}))
            }
            _ => Err(InvokeError::BadRequest("unknown operation".to_owned())),
        }
    }
}

fn map_contract_error(error: apparatus_contracts::ApparatusError) -> InvokeError {
    match error {
        apparatus_contracts::ApparatusError::PayloadTooLarge { .. } => InvokeError::PayloadTooLarge,
        other => InvokeError::BadRequest(stable_kv_reason(&other)),
    }
}

fn map_kv_put_error(error: apparatus_contracts::ApparatusError) -> InvokeError {
    match error {
        apparatus_contracts::ApparatusError::PayloadTooLarge { .. } => InvokeError::PayloadTooLarge,
        apparatus_contracts::ApparatusError::InvalidOperation { ref reason }
            if reason == "kv store failed" =>
        {
            InvokeError::Failed
        }
        apparatus_contracts::ApparatusError::InvalidOperation { reason } => {
            InvokeError::BadRequest(reason)
        }
        apparatus_contracts::ApparatusError::InvalidId { reason } => {
            InvokeError::BadRequest(reason)
        }
        _ => InvokeError::Failed,
    }
}

fn stable_kv_reason(error: &apparatus_contracts::ApparatusError) -> String {
    match error {
        apparatus_contracts::ApparatusError::InvalidOperation { reason } => reason.clone(),
        apparatus_contracts::ApparatusError::InvalidId { reason } => reason.clone(),
        other => other.to_string(),
    }
}

fn capability_for_operation(operation: &str) -> Result<Capability, InvokeError> {
    match operation {
        "kv.get" => Ok(Capability::StorageKvRead),
        "kv.put" | "kv.delete" => Ok(Capability::StorageKvWrite),
        "connector.fetch" => Ok(Capability::ConnectorFetch),
        other => Err(InvokeError::BadRequest(format!(
            "unknown operation {other}"
        ))),
    }
}

fn param_str<'a>(params: &'a serde_json::Value, field: &str) -> Result<&'a str, InvokeError> {
    params
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| InvokeError::BadRequest(format!("params must carry {field}")))
}

fn reject_urlish(params: &serde_json::Value) -> Result<(), InvokeError> {
    let Some(obj) = params.as_object() else {
        return Ok(());
    };
    for (key, value) in obj {
        let lowered = key.to_ascii_lowercase();
        if lowered.contains("url") || lowered == "fetchinternal" || lowered.contains("href") {
            return Err(InvokeError::BadRequest("raw url rejected".to_owned()));
        }
        if let Some(text) = value.as_str() {
            if text.contains("://") || text.eq_ignore_ascii_case("fetchInternal") {
                return Err(InvokeError::BadRequest("raw url rejected".to_owned()));
            }
        }
    }
    Ok(())
}

/// Documented P0 path reused by Lazaret HTTP.
#[must_use]
pub const fn invoke_path() -> &'static str {
    INVOKE_PATH
}

fn plugin_invoke_url(endpoint: &str) -> String {
    format!("{}{INVOKE_PATH}", endpoint.trim_end_matches('/'))
}
