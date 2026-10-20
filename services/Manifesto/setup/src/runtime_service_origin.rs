//! Origin verifier configured once alongside the actual standalone TLS listener.
//! Chain validation/client-key possession belongs to the existing RustyCog
//! acceptor. Parsing a DER or a test extension alone is never transport proof.

use apparatus_contracts::runtime::{
    RuntimeControlCallerV1, RuntimeControlErrorV1, RuntimeEffectOutcomeV1, RuntimeErrorCodeV1,
    RuntimeServiceRoleV1,
};
use manifesto_http_server::{
    RejectRuntimeServiceOrigin, RuntimeServiceOriginAuth, RuntimeServiceOriginAuthenticator,
};
use rustycog::{config::ServerConfig, http::PeerClientCertificate};
use std::sync::Arc;
use x509_parser::extensions::GeneralName;

pub(crate) fn build_runtime_service_origin(
    config: &ServerConfig,
) -> anyhow::Result<RuntimeServiceOriginAuth> {
    if !config.tls_enabled || config.tls_client_ca_path.trim().is_empty() {
        return Ok(Arc::new(RejectRuntimeServiceOrigin));
    }
    // Do not install an apparently enabled verifier for an unavailable CA. The
    // same explicit path is consumed by serve_router's client-cert verifier.
    let pem = std::fs::read(&config.tls_client_ca_path)
        .map_err(|_| anyhow::anyhow!("runtime service client CA is unavailable"))?;
    let mut count = 0;
    for block in x509_parser::pem::Pem::iter_from_buffer(&pem) {
        let block = block.map_err(|_| anyhow::anyhow!("runtime service client CA is malformed"))?;
        if block.label != "CERTIFICATE" {
            return Err(anyhow::anyhow!(
                "runtime service client CA must contain certificates only"
            ));
        }
        let (rest, certificate) = x509_parser::parse_x509_certificate(&block.contents)
            .map_err(|_| anyhow::anyhow!("runtime service client CA is malformed"))?;
        if !rest.is_empty()
            || !certificate
                .basic_constraints()
                .ok()
                .flatten()
                .is_some_and(|extension| extension.value.ca)
        {
            return Err(anyhow::anyhow!(
                "runtime service client trust requires an explicit CA certificate"
            ));
        }
        count += 1;
    }
    if count == 0 {
        return Err(anyhow::anyhow!("runtime service client CA is empty"));
    }
    Ok(Arc::new(AcceptedServiceLeafOrigin))
}

struct AcceptedServiceLeafOrigin;
impl RuntimeServiceOriginAuthenticator for AcceptedServiceLeafOrigin {
    fn authenticate(
        &self,
        peer: Option<&PeerClientCertificate>,
    ) -> Result<RuntimeControlCallerV1, RuntimeControlErrorV1> {
        let unauthenticated = || failure(RuntimeErrorCodeV1::RuntimeUnauthenticated);
        let peer = peer.ok_or_else(unauthenticated)?;
        let (rest, certificate) =
            x509_parser::parse_x509_certificate(&peer.der).map_err(|_| unauthenticated())?;
        if !rest.is_empty()
            || !certificate.validity().is_valid()
            || std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .is_err()
            || !certificate
                .extended_key_usage()
                .ok()
                .flatten()
                .is_some_and(|extension| extension.value.client_auth)
            || certificate
                .basic_constraints()
                .ok()
                .flatten()
                .is_some_and(|extension| extension.value.ca)
        {
            return Err(unauthenticated());
        }
        let san = certificate
            .subject_alternative_name()
            .map_err(|_| unauthenticated())?
            .ok_or_else(unauthenticated)?;
        let mut uris = san
            .value
            .general_names
            .iter()
            .filter_map(|name| match name {
                GeneralName::URI(uri) => Some(*uri),
                _ => None,
            });
        let principal = uris.next().ok_or_else(unauthenticated)?;
        if uris.next().is_some() {
            return Err(unauthenticated());
        }
        let role = RuntimeServiceRoleV1::from_verified_principal(principal).map_err(failure)?;
        Ok(RuntimeControlCallerV1 { role })
    }
}

fn failure(code: RuntimeErrorCodeV1) -> RuntimeControlErrorV1 {
    code.error(
        RuntimeServiceRoleV1::Projector,
        RuntimeEffectOutcomeV1::NotStarted,
    )
}
