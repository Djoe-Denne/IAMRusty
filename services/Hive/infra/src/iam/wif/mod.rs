//! OIDC Workload Identity Federation adapters (ADR-0307).
//!
//! AWS / GCP / Azure exchange a file-backed JWT for a short-lived secret.
//! Static fallback only when `provider` is absent or `static`.

mod aws;
mod azure;
mod gcp;

pub use aws::AwsWif;
pub use azure::AzureWif;
pub use gcp::GcpWif;

use crate::iam::StaticCredential;
use hive_configuration::{
    AwsWorkloadConfig, AzureWorkloadConfig, GcpWorkloadConfig, WorkloadIdentityConfig,
};
use hive_domain::port::service::WorkloadIdentity;
use rustycog::core::error::DomainError;
use std::sync::Arc;

fn require_non_empty(field: &str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        return Err(DomainError::external_service_error(
            "workload_identity",
            &format!("wif config incomplete: {field} must not be empty"),
        ));
    }
    Ok(())
}

fn incomplete(provider: &str) -> DomainError {
    DomainError::external_service_error(
        "workload_identity",
        &format!("wif config incomplete: provider={provider} missing nested block"),
    )
}

/// Validate AWS WIF required fields (fail-closed at composition).
///
/// # Errors
///
/// Returns [`DomainError`] when any required field is empty.
pub fn validate_aws_config(config: &AwsWorkloadConfig) -> Result<(), DomainError> {
    require_non_empty("aws.token_url", &config.token_url)?;
    require_non_empty("aws.subject_token_file", &config.subject_token_file)?;
    require_non_empty("aws.role_arn", &config.role_arn)?;
    require_non_empty("aws.role_session_name", &config.role_session_name)?;
    require_non_empty("aws.audience", &config.audience)?;
    Ok(())
}

/// Validate GCP WIF required fields (fail-closed at composition).
///
/// # Errors
///
/// Returns [`DomainError`] when any required field is empty.
pub fn validate_gcp_config(config: &GcpWorkloadConfig) -> Result<(), DomainError> {
    require_non_empty("gcp.token_url", &config.token_url)?;
    require_non_empty("gcp.subject_token_file", &config.subject_token_file)?;
    require_non_empty("gcp.audience", &config.audience)?;
    Ok(())
}

/// Validate Azure WIF required fields (fail-closed at composition).
///
/// # Errors
///
/// Returns [`DomainError`] when any required field is empty.
pub fn validate_azure_config(config: &AzureWorkloadConfig) -> Result<(), DomainError> {
    require_non_empty("azure.token_url", &config.token_url)?;
    require_non_empty("azure.subject_token_file", &config.subject_token_file)?;
    require_non_empty("azure.tenant_id", &config.tenant_id)?;
    require_non_empty("azure.client_id", &config.client_id)?;
    require_non_empty("azure.scope", &config.scope)?;
    Ok(())
}

/// Compose a [`WorkloadIdentity`] from optional WIF config + static secret fallback.
///
/// - `provider` absent / `static` → [`StaticCredential`] (empty static secret = existing fail-closed)
/// - `aws` | `gcp` | `azure` with missing nested fields → [`Err`] (no silent static fallback)
///
/// # Errors
///
/// Returns [`DomainError`] on incomplete WIF config, unknown provider, or empty static secret.
pub fn compose_workload_identity(
    workload: Option<&WorkloadIdentityConfig>,
    static_secret: &str,
    credential_ref: &str,
) -> Result<Arc<dyn WorkloadIdentity>, DomainError> {
    let provider = workload
        .and_then(|w| w.provider.as_deref())
        .unwrap_or("static")
        .trim()
        .to_ascii_lowercase();

    match provider.as_str() {
        "" | "static" => {
            let static_cred = StaticCredential::from_pair(credential_ref, static_secret)?;
            Ok(Arc::new(static_cred) as Arc<dyn WorkloadIdentity>)
        }
        "aws" => {
            let cfg = workload
                .and_then(|w| w.aws.as_ref())
                .ok_or_else(|| incomplete("aws"))?;
            validate_aws_config(cfg)?;
            Ok(Arc::new(AwsWif::new(cfg)?) as Arc<dyn WorkloadIdentity>)
        }
        "gcp" => {
            let cfg = workload
                .and_then(|w| w.gcp.as_ref())
                .ok_or_else(|| incomplete("gcp"))?;
            validate_gcp_config(cfg)?;
            Ok(Arc::new(GcpWif::new(cfg)?) as Arc<dyn WorkloadIdentity>)
        }
        "azure" => {
            let cfg = workload
                .and_then(|w| w.azure.as_ref())
                .ok_or_else(|| incomplete("azure"))?;
            validate_azure_config(cfg)?;
            Ok(Arc::new(AzureWif::new(cfg)?) as Arc<dyn WorkloadIdentity>)
        }
        other => Err(DomainError::external_service_error(
            "workload_identity",
            &format!("unknown workload provider: {other}"),
        )),
    }
}

pub(crate) fn read_subject_token(path: &str) -> Result<String, DomainError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        DomainError::external_service_error(
            "workload_identity",
            &format!("failed to read subject_token_file: {e}"),
        )
    })?;
    let token = raw.trim();
    if token.is_empty() {
        return Err(DomainError::external_service_error(
            "workload_identity",
            "subject_token_file is empty",
        ));
    }
    Ok(token.to_string())
}

pub(crate) fn extract_xml_tag(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    let value = body[start..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod compose_tests {
    use super::*;
    use hive_configuration::AwsWorkloadConfig;

    #[test]
    fn compose_workload_static_when_unset() {
        let wi = compose_workload_identity(None, "s.static", "iam-internal-token").expect("static");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let cred = rt
            .block_on(wi.resolve("iam-internal-token"))
            .expect("resolve static");
        assert_eq!(cred.secret, "s.static");
    }

    #[test]
    fn compose_workload_aws_incomplete_is_fail_closed() {
        let cfg = WorkloadIdentityConfig {
            provider: Some("aws".into()),
            aws: Some(AwsWorkloadConfig {
                token_url: "http://127.0.0.1:3000".into(),
                subject_token_file: "config/test-oidc.jwt".into(),
                role_arn: String::new(),
                role_session_name: "sess".into(),
                audience: "aud".into(),
            }),
            gcp: None,
            azure: None,
        };
        let result = compose_workload_identity(Some(&cfg), "s.static", "iam-internal-token");
        assert!(result.is_err(), "incomplete aws must fail-closed");
        let msg = result.err().expect("err").to_string();
        assert!(
            msg.contains("incomplete") || msg.contains("role_arn"),
            "unexpected err: {msg}"
        );
    }

    #[test]
    fn compose_workload_gcp_missing_block_is_fail_closed() {
        let cfg = WorkloadIdentityConfig {
            provider: Some("gcp".into()),
            aws: None,
            gcp: None,
            azure: None,
        };
        let result = compose_workload_identity(Some(&cfg), "s.static", "iam-internal-token");
        assert!(result.is_err(), "missing gcp block must fail-closed");
        let msg = result.err().expect("err").to_string();
        assert!(
            msg.contains("incomplete") || msg.contains("gcp"),
            "unexpected err: {msg}"
        );
    }

    #[test]
    fn compose_workload_unknown_provider_is_err() {
        let cfg = WorkloadIdentityConfig {
            provider: Some("not-a-cloud".into()),
            aws: None,
            gcp: None,
            azure: None,
        };
        let result = compose_workload_identity(Some(&cfg), "s.static", "iam-internal-token");
        assert!(result.is_err(), "unknown provider must be err");
        let msg = result.err().expect("err").to_string();
        assert!(msg.contains("unknown"), "unexpected err: {msg}");
    }
}
