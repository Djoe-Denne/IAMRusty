//! Protected HTTP adapter to the Manifesto runtime writer (no SQL/cluster client).
//!
//! The composition root alone supplies the fixed URL and service mTLS identity.
//! Metadata and commands are sent once; a transport failure on POST is ambiguous.

use std::time::Duration;

use apparatus_contracts::runtime::*;
use reqwest::{Method, StatusCode, Url};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

pub struct HttpRuntimeControl {
    base: Url,
    client: reqwest::Client,
}

/// No network authority in the runtime-off profile; never an allow fallback.
pub struct DisabledRuntimeControl;

fn disabled<'a, T: Send + 'a>() -> RuntimeControlFuture<'a, T> {
    Box::pin(async {
        Err(local_error(
            RuntimeErrorCodeV1::RuntimeAuthorityUnavailable,
            false,
        ))
    })
}

impl RuntimeControlPort for DisabledRuntimeControl {
    fn list_bindings<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Option<&'a RuntimeCursorV1>,
        _: Option<u16>,
    ) -> RuntimeControlFuture<'a, RuntimeBindingPageV1> {
        disabled()
    }
    fn get_binding<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeInstanceSelectorV1,
    ) -> RuntimeControlFuture<'a, RuntimeBindingSnapshotV1> {
        disabled()
    }
    fn writer_lease<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeWriterLeaseRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriterLeaseV1> {
        disabled()
    }
    fn report_observation<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeObservationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        disabled()
    }
    fn register_instance<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeInstanceRegistrationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        disabled()
    }
    fn revoke_instance<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeInstanceRevokeRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        disabled()
    }
    fn begin_effect<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeBeginEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeEffectReservationV1> {
        disabled()
    }
    fn complete_effect<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeCompleteEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        disabled()
    }
}

impl HttpRuntimeControl {
    /// Build from platform-controlled configuration, never caller request fields.
    pub fn new(
        base_url: &str,
        identity: reqwest::Identity,
        server_ca: reqwest::Certificate,
    ) -> Result<Self, RuntimeErrorCodeV1> {
        let mut base = Url::parse(base_url).map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
        if base.scheme() != "https"
            || base.host_str().is_none()
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(RuntimeErrorCodeV1::RuntimeInternal);
        }
        let path = format!(
            "{}/internal/apparatus/runtime/",
            base.path().trim_end_matches('/')
        );
        base.set_path(&path);
        let client = reqwest::Client::builder()
            // Identity::from_pem is rustls-specific. With reqwest's default
            // native-tls feature also enabled, backend selection is not implicit.
            .use_rustls_tls()
            .identity(identity)
            .add_root_certificate(server_ca)
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| {
                use std::error::Error as _;
                // Builder errors have no request URL. Preserve the TLS cause
                // without serializing the identity, CA, PEM or any credential.
                if let Some(cause) = error.source() {
                    tracing::error!(component = "runtime_control", phase = "client_build",
                        cause = %cause, "runtime_transport_configuration_failed");
                } else {
                    tracing::error!(component = "runtime_control", phase = "client_build",
                        cause = %error, "runtime_transport_configuration_failed");
                }
                RuntimeErrorCodeV1::RuntimeInternal
            })?;
        Ok(Self { base, client })
    }

    async fn send<T: DeserializeOwned, R: Serialize + Sync>(
        &self,
        caller: &RuntimeControlCallerV1,
        method: Method,
        project_id: Uuid,
        path: &str,
        query: &[(&str, String)],
        body: Option<&R>,
    ) -> Result<T, RuntimeControlErrorV1> {
        if caller.role != RuntimeServiceRoleV1::Lazaret {
            return Err(local_error(
                RuntimeErrorCodeV1::RuntimeServiceForbidden,
                false,
            ));
        }
        let ambiguous = method != Method::GET;
        let url = self
            .base
            .join(path)
            .map_err(|_| local_error(RuntimeErrorCodeV1::RuntimeInternal, false))?;
        let mut request = self
            .client
            .request(method, url)
            .query(&[("project_id", project_id.to_string())])
            .query(query);
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| transport_error(ambiguous))?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| transport_error(ambiguous))?
        {
            if chunk.len() > MAX_RUNTIME_BODY_BYTES.saturating_sub(bytes.len()) {
                return Err(local_error(
                    RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                    ambiguous,
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if status != StatusCode::OK {
            let error = parse_runtime_json::<RuntimeControlErrorV1>(&bytes).map_err(|_| {
                local_error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError, ambiguous)
            })?;
            if error.code.http_status() != status.as_u16() {
                return Err(local_error(
                    RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                    ambiguous,
                ));
            }
            // Never act on retry_class for a command. It is metadata, not an
            // instruction to re-deliver a possibly committed operation.
            return Err(error);
        }
        parse_runtime_json::<T>(&bytes)
            .map_err(|_| local_error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError, ambiguous))
    }
}

fn local_error(code: RuntimeErrorCodeV1, ambiguous: bool) -> RuntimeControlErrorV1 {
    let outcome = if ambiguous {
        RuntimeEffectOutcomeV1::MayHaveStarted
    } else {
        RuntimeEffectOutcomeV1::NotStarted
    };
    code.error(RuntimeServiceRoleV1::Lazaret, outcome)
}

fn transport_error(ambiguous: bool) -> RuntimeControlErrorV1 {
    local_error(
        if ambiguous {
            RuntimeErrorCodeV1::RuntimeEffectOutcomeUnknown
        } else {
            RuntimeErrorCodeV1::RuntimeAuthorityUnavailable
        },
        ambiguous,
    )
}

fn forbidden<'a, T: Send + 'a>() -> RuntimeControlFuture<'a, T> {
    Box::pin(async {
        Err(local_error(
            RuntimeErrorCodeV1::RuntimeServiceForbidden,
            false,
        ))
    })
}

impl RuntimeControlPort for HttpRuntimeControl {
    fn list_bindings<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        cursor: Option<&'a RuntimeCursorV1>,
        page_size: Option<u16>,
    ) -> RuntimeControlFuture<'a, RuntimeBindingPageV1> {
        Box::pin(async move {
            let mut query = vec![("schema_version", "1".to_owned())];
            if let Some(cursor) = cursor {
                query.push(("cursor", cursor.0.clone()));
            }
            if let Some(size) = page_size {
                query.push(("page_size", size.to_string()));
            }
            let page: RuntimeBindingPageV1 = self
                .send::<_, ()>(caller, Method::GET, project_id, "bindings", &query, None)
                .await?;
            if page.items.len() > 200 || page.items.iter().any(|item| item.project_id != project_id)
            {
                return Err(local_error(
                    RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                    false,
                ));
            }
            Ok(page)
        })
    }

    fn get_binding<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instances: &'a RuntimeInstanceSelectorV1,
    ) -> RuntimeControlFuture<'a, RuntimeBindingSnapshotV1> {
        Box::pin(async move {
            let mut query = vec![("schema_version", "1".to_owned())];
            match instances {
                RuntimeInstanceSelectorV1::Page { cursor, page_size } => {
                    query.push(("instances_mode", "page".to_owned()));
                    if let Some(cursor) = cursor {
                        query.push(("instances_cursor", cursor.0.clone()));
                    }
                    if let Some(size) = page_size {
                        query.push(("instances_page_size", size.to_string()));
                    }
                }
                RuntimeInstanceSelectorV1::ByPodUid { pod_uid } => {
                    query.push(("instances_mode", "by_pod_uid".to_owned()));
                    query.push(("pod_uid", pod_uid.clone()));
                }
                RuntimeInstanceSelectorV1::ByInstanceId { instance_id } => {
                    query.push(("instances_mode", "by_instance_id".to_owned()));
                    query.push(("instance_id", instance_id.to_string()));
                }
            }
            let snapshot: RuntimeBindingSnapshotV1 = self
                .send::<_, ()>(
                    caller,
                    Method::GET,
                    project_id,
                    &format!("bindings/{binding_id}"),
                    &query,
                    None,
                )
                .await?;
            if snapshot.project_id != project_id || snapshot.binding_id != binding_id {
                return Err(local_error(
                    RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                    false,
                ));
            }
            let page = snapshot.instances.as_ref().ok_or_else(|| {
                local_error(RuntimeErrorCodeV1::RuntimeAuthorityProtocolError, false)
            })?;
            if page.items.len() > 200
                || page.items.iter().any(|instance| {
                    instance.project_id != project_id || instance.binding_id != binding_id
                })
            {
                return Err(local_error(
                    RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                    false,
                ));
            }
            match instances {
                RuntimeInstanceSelectorV1::Page { .. } => {}
                RuntimeInstanceSelectorV1::ByPodUid { pod_uid } => {
                    if page.next_cursor.is_some()
                        || page.items.len() > 1
                        || page
                            .items
                            .iter()
                            .any(|item| item.pod.as_ref().is_none_or(|pod| &pod.pod_uid != pod_uid))
                    {
                        return Err(local_error(
                            RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                            false,
                        ));
                    }
                }
                RuntimeInstanceSelectorV1::ByInstanceId { instance_id } => {
                    if page.next_cursor.is_some()
                        || page.items.len() > 1
                        || page
                            .items
                            .iter()
                            .any(|item| &item.instance_id != instance_id)
                    {
                        return Err(local_error(
                            RuntimeErrorCodeV1::RuntimeAuthorityProtocolError,
                            false,
                        ));
                    }
                }
            }
            Ok(snapshot)
        })
    }

    fn writer_lease<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeWriterLeaseRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriterLeaseV1> {
        forbidden()
    }
    fn report_observation<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeObservationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        forbidden()
    }
    fn register_instance<'a>(
        &'a self,
        _: &'a RuntimeControlCallerV1,
        _: Uuid,
        _: Uuid,
        _: Uuid,
        _: &'a RuntimeInstanceRegistrationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        forbidden()
    }

    fn revoke_instance<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &'a RuntimeInstanceRevokeRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            self.send(
                caller,
                Method::POST,
                project_id,
                &format!("bindings/{binding_id}/instances/{instance_id}/revoke"),
                &[],
                Some(request),
            )
            .await
        })
    }
    fn begin_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeBeginEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeEffectReservationV1> {
        Box::pin(async move {
            self.send(
                caller,
                Method::POST,
                project_id,
                &format!("bindings/{binding_id}/effects/begin"),
                &[],
                Some(request),
            )
            .await
        })
    }
    fn complete_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        effect_id: Uuid,
        request: &'a RuntimeCompleteEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1> {
        Box::pin(async move {
            self.send(
                caller,
                Method::POST,
                project_id,
                &format!("bindings/{binding_id}/effects/{effect_id}/complete"),
                &[],
                Some(request),
            )
            .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_adapter_accepts_pem_identity_with_both_tls_backends_enabled() {
        // Constructor regression only: no socket, server trust, or mTLS
        // authorization is claimed by building this local test identity.
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("test certificate");
        let certificate = issued.cert.pem();
        let pem = format!("{certificate}{}", issued.key_pair.serialize_pem());
        let identity = reqwest::Identity::from_pem(pem.as_bytes()).expect("PEM identity");
        let ca = reqwest::Certificate::from_pem(certificate.as_bytes()).expect("CA syntax");
        assert!(HttpRuntimeControl::new("https://localhost/manifesto", identity, ca).is_ok());
    }

    #[test]
    fn command_transport_failure_is_ambiguous_and_never_retryable() {
        let error = transport_error(true);
        assert_eq!(error.code, RuntimeErrorCodeV1::RuntimeEffectOutcomeUnknown);
        assert_eq!(error.effect_outcome, RuntimeEffectOutcomeV1::MayHaveStarted);
        assert_eq!(error.retry_class, RuntimeRetryClassV1::Never);
    }

    #[test]
    fn metadata_failure_does_not_claim_an_effect_started() {
        let error = transport_error(false);
        assert_eq!(error.code, RuntimeErrorCodeV1::RuntimeAuthorityUnavailable);
        assert_eq!(error.effect_outcome, RuntimeEffectOutcomeV1::NotStarted);
        assert_eq!(error.retry_class, RuntimeRetryClassV1::ReadOnly);
    }
}
