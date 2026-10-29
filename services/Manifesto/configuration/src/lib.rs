//! Manifesto Service Configuration
//!
//! This crate provides configuration structures for the Manifesto service,
//! including service-specific configurations and integration with rustycog-config.

use serde::{Deserialize, Serialize};

use rustycog::config::{
    load_config_fresh, AuthConfig, CommandConfig, ConfigError, ConfigLoader, DatabaseConfig,
    HasDbConfig, HasLoggingConfig, HasOpenFgaConfig, HasQueueConfig, HasScalewayConfig,
    HasServerConfig, LoggingConfig, OpenFgaClientConfig, QueueConfig, ScalewayConfig, ServerConfig,
};

pub use rustycog::logger::setup_logging;

/// Type alias for backward compatibility
pub type AppConfig = ManifestoConfig;

/// Main Manifesto service configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestoConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Shared authentication verifier configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Command execution configuration
    #[serde(default)]
    pub command: CommandConfig,

    /// Queue configuration (SQS, Kafka, etc.)
    #[serde(default)]
    pub queue: QueueConfig,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// Scaleway configuration
    #[serde(default)]
    pub scaleway: ScalewayConfig,

    /// Service-specific configuration
    #[serde(default)]
    pub service: ServiceConfig,

    /// `OpenFGA` authorization checker configuration.
    #[serde(default)]
    pub openfga: OpenFgaClientConfig,

    /// Apparatus binding grant snapshot settings (distinct from `[auth.jwt]`).
    #[serde(default)]
    pub apparatus: ApparatusConfig,
}

/// Apparatus-specific Manifesto configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApparatusConfig {
    /// HS256 verifier for the privileged binding grant snapshot GET.
    #[serde(default)]
    pub grant_snapshot_auth: GrantSnapshotAuthConfig,
    #[serde(default)]
    pub runtime_control: RuntimeControlConfig,
}

/// Separate explicit runtime mode. False is not permission to ignore persisted
/// fences: setup refuses that combination instead of restarting a legacy path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeControlConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub maintenance_enabled: bool,
    #[serde(default)]
    pub authority_id: Option<String>,
    #[serde(default)]
    pub installation_id: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub profile_lock_sha256: Option<String>,
    #[serde(default)]
    pub endpoint_scheme: Option<String>,
    #[serde(default)]
    pub endpoint_port: Option<u16>,
    #[serde(default)]
    pub allowed_repositories: Vec<String>,
    #[serde(default)]
    pub admission_keys: Vec<RuntimeAdmissionPublicKeyConfig>,
}

/// Public trust only; never private-key/Transit/JWKS discovery configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeAdmissionPublicKeyConfig {
    pub authority_id: String,
    pub installation_id: String,
    pub issuer: String,
    pub audience: String,
    pub purpose: String,
    pub kid: String,
    pub public_key_pem_path: String,
}

/// Platform-service JWT for `GET /api/projects/{project_id}/bindings/{component_id}`.
/// Distinct from IAM `[auth.jwt]` (`iss=iamrusty`, `aud=aiforall`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSnapshotAuthConfig {
    /// HS256 secret shared with platform callers of the snapshot GET.
    #[serde(default)]
    pub hs256_secret: Option<String>,
    /// Expected `iss` claim.
    #[serde(default = "default_grant_snapshot_issuer")]
    pub issuer: String,
    /// Expected `aud` claim.
    #[serde(default = "default_grant_snapshot_audience")]
    pub audience: String,
    /// Explicit authority attestation for this service-only UUID actor view.
    /// Never inherited from the IAM policy or inferred from the HS256 issuer.
    #[serde(default)]
    pub service_principal_issuer: Option<String>,
}

impl Default for GrantSnapshotAuthConfig {
    fn default() -> Self {
        Self {
            hs256_secret: None,
            issuer: default_grant_snapshot_issuer(),
            audience: default_grant_snapshot_audience(),
            service_principal_issuer: None,
        }
    }
}

/// Opaque startup failure; configuration values and key material are not exposed.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Invalid grant snapshot auth configuration")]
pub struct GrantSnapshotAuthConfigError;

impl GrantSnapshotAuthConfig {
    /// Validate the dedicated service verifier without normalizing authority names.
    ///
    /// # Errors
    ///
    /// Returns an opaque error for missing key material, invalid issuer/audience,
    /// or an absent, invalid or mismatched explicit service authority.
    pub fn validate(&self) -> Result<(), GrantSnapshotAuthConfigError> {
        let issuer = self
            .service_principal_issuer
            .as_ref()
            .ok_or(GrantSnapshotAuthConfigError)?;
        rustycog::permission::PrincipalPolicy::new(issuer.clone(), false)
            .map_err(|_| GrantSnapshotAuthConfigError)?;
        if issuer != &self.issuer
            || self.issuer.is_empty()
            || self.issuer.trim() != self.issuer
            || self.audience.is_empty()
            || self.audience.trim() != self.audience
            || self
                .hs256_secret
                .as_ref()
                .is_none_or(|secret| secret.trim().is_empty())
        {
            return Err(GrantSnapshotAuthConfigError);
        }
        Ok(())
    }
}

fn default_grant_snapshot_issuer() -> String {
    "aiforall-platform".to_owned()
}

fn default_grant_snapshot_audience() -> String {
    "manifesto-bindings".to_owned()
}

/// Component service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentServiceConfig {
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

const fn default_timeout_seconds() -> u64 {
    10
}

impl Default for ComponentServiceConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:9000".to_string(),
            api_key: None,
            timeout_seconds: default_timeout_seconds(),
        }
    }
}

/// Service-specific configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceConfig {
    /// Component service configuration
    #[serde(default)]
    pub component_service: ComponentServiceConfig,

    /// Business logic configuration
    #[serde(default)]
    pub business: BusinessConfig,
}

/// Business logic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessConfig {
    /// Maximum projects per user
    #[serde(default = "default_max_projects_per_user")]
    pub max_projects_per_user: u32,

    /// Maximum projects per organization
    #[serde(default = "default_max_projects_per_org")]
    pub max_projects_per_org: u32,

    /// Maximum members per project
    #[serde(default = "default_max_members_per_project")]
    pub max_members_per_project: u32,

    /// Maximum components per project
    #[serde(default = "default_max_components_per_project")]
    pub max_components_per_project: u32,

    /// Default page size for pagination
    #[serde(default = "default_page_size")]
    pub default_page_size: u32,

    /// Maximum page size for pagination
    #[serde(default = "default_max_page_size")]
    pub max_page_size: u32,

    /// Project name maximum length
    #[serde(default = "default_project_name_max_length")]
    pub project_name_max_length: usize,

    /// Project description maximum length
    #[serde(default = "default_project_description_max_length")]
    pub project_description_max_length: usize,

    /// Grace period for member removal (in days)
    #[serde(default = "default_member_removal_grace_period_days")]
    pub member_removal_grace_period_days: u32,
}

// Default value functions
const fn default_true() -> bool {
    true
}
fn default_email_provider() -> String {
    "dummy".to_string()
}
fn default_from_email() -> String {
    "noreply@manifesto.com".to_string()
}
fn default_from_name() -> String {
    "Manifesto Service".to_string()
}
fn default_notification_provider() -> String {
    "dummy".to_string()
}
fn default_cache_provider() -> String {
    "memory".to_string()
}
const fn default_cache_ttl() -> u32 {
    3600
} // 1 hour
const fn default_max_projects_per_user() -> u32 {
    100
}
const fn default_max_projects_per_org() -> u32 {
    500
}
const fn default_max_members_per_project() -> u32 {
    100
}
const fn default_max_components_per_project() -> u32 {
    50
}
const fn default_page_size() -> u32 {
    20
}
const fn default_max_page_size() -> u32 {
    100
}
const fn default_project_name_max_length() -> usize {
    255
}
const fn default_project_description_max_length() -> usize {
    2000
}
const fn default_member_removal_grace_period_days() -> u32 {
    30
}

// Default implementations

impl Default for BusinessConfig {
    fn default() -> Self {
        Self {
            max_projects_per_user: default_max_projects_per_user(),
            max_projects_per_org: default_max_projects_per_org(),
            max_members_per_project: default_max_members_per_project(),
            max_components_per_project: default_max_components_per_project(),
            default_page_size: default_page_size(),
            max_page_size: default_max_page_size(),
            project_name_max_length: default_project_name_max_length(),
            project_description_max_length: default_project_description_max_length(),
            member_removal_grace_period_days: default_member_removal_grace_period_days(),
        }
    }
}

// Trait implementations for rustycog-config integration
impl ConfigLoader<Self> for ManifestoConfig {
    fn create_default() -> Self {
        Self::default()
    }

    fn config_prefix() -> &'static str {
        "MANIFESTO"
    }
}

impl HasServerConfig for ManifestoConfig {
    fn server_config(&self) -> &ServerConfig {
        &self.server
    }

    fn set_server_config(&mut self, config: ServerConfig) {
        self.server = config;
    }
}

impl HasLoggingConfig for ManifestoConfig {
    fn logging_config(&self) -> &LoggingConfig {
        &self.logging
    }

    fn set_logging_config(&mut self, config: LoggingConfig) {
        self.logging = config;
    }
}

impl HasScalewayConfig for ManifestoConfig {
    fn scaleway_config(&self) -> &ScalewayConfig {
        &self.scaleway
    }

    fn set_scaleway_config(&mut self, config: ScalewayConfig) {
        self.scaleway = config;
    }
}

impl HasQueueConfig for ManifestoConfig {
    fn queue_config(&self) -> &QueueConfig {
        &self.queue
    }

    fn set_queue_config(&mut self, config: QueueConfig) {
        self.queue = config;
    }
}

impl HasDbConfig for ManifestoConfig {
    fn db_config(&self) -> &DatabaseConfig {
        &self.database
    }

    fn set_db_config(&mut self, config: DatabaseConfig) {
        self.database = config;
    }
}

impl HasOpenFgaConfig for ManifestoConfig {
    fn openfga_config(&self) -> &OpenFgaClientConfig {
        &self.openfga
    }

    fn set_openfga_config(&mut self, config: OpenFgaClientConfig) {
        self.openfga = config;
    }
}

/// Load configuration from environment and config files
///
/// # Errors
///
/// Returns [`ConfigError`] when required files or environment values cannot be loaded.
pub fn load_config() -> Result<ManifestoConfig, ConfigError> {
    load_config_fresh::<ManifestoConfig>()
}

#[cfg(test)]
mod grant_snapshot_policy_tests {
    use super::*;

    #[test]
    fn grant_snapshot_requires_explicit_service_principal_policy() {
        let valid = GrantSnapshotAuthConfig {
            hs256_secret: Some("fixture-only-key".to_owned()),
            service_principal_issuer: Some("aiforall-platform".to_owned()),
            ..GrantSnapshotAuthConfig::default()
        };
        assert!(valid.validate().is_ok());
        assert!(GrantSnapshotAuthConfig::default()
            .service_principal_issuer
            .is_none());
        let deserialized: GrantSnapshotAuthConfig = toml::from_str("").unwrap();
        assert!(deserialized.service_principal_issuer.is_none());
        assert!(deserialized.validate().is_err());
        for issuer in ["custom-service".to_owned(), "é".repeat(128)] {
            assert!(GrantSnapshotAuthConfig {
                issuer: issuer.clone(),
                service_principal_issuer: Some(issuer),
                ..valid.clone()
            }
            .validate()
            .is_ok());
        }

        for issuer in [
            None,
            Some(String::new()),
            Some(" ".to_owned()),
            Some(" aiforall-platform".to_owned()),
            Some("aiforall-platform ".to_owned()),
            Some("https://platform.example/iam".to_owned()),
            Some("x".repeat(257)),
            Some("é".repeat(129)),
        ] {
            let config = GrantSnapshotAuthConfig {
                service_principal_issuer: issuer,
                ..valid.clone()
            };
            let error = config.validate().unwrap_err();
            assert_eq!(
                error.to_string(),
                "Invalid grant snapshot auth configuration"
            );
            assert_eq!(format!("{error:?}"), "GrantSnapshotAuthConfigError");
        }
        // Even equal crypto/policy fields must pass the shared authority validator.
        for issuer in ["x".repeat(257), "é".repeat(129), " padded ".to_owned()] {
            assert!(GrantSnapshotAuthConfig {
                issuer: issuer.clone(),
                service_principal_issuer: Some(issuer),
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
        for value in ["", " ", " padded", "padded "] {
            for config in [
                GrantSnapshotAuthConfig {
                    issuer: value.to_owned(),
                    ..valid.clone()
                },
                GrantSnapshotAuthConfig {
                    audience: value.to_owned(),
                    ..valid.clone()
                },
            ] {
                assert_eq!(
                    config.validate().unwrap_err().to_string(),
                    "Invalid grant snapshot auth configuration"
                );
            }
        }
        for secret in [None, Some(String::new()), Some(" \t".to_owned())] {
            assert!(GrantSnapshotAuthConfig {
                hs256_secret: secret,
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
        // All three shipped configurations explicitly attest the service authority,
        // without borrowing the different global HumanAccount authority.
        for source in [
            include_str!("../../config/default.toml"),
            include_str!("../../config/development.toml"),
            include_str!("../../config/test.toml"),
        ] {
            let config: ManifestoConfig = toml::from_str(source).unwrap();
            let service = &config.apparatus.grant_snapshot_auth;
            assert_eq!(
                service.service_principal_issuer.as_deref(),
                Some("aiforall-platform")
            );
            assert!(service.validate().is_ok());
            assert_ne!(
                config.auth.jwt.platform_issuer.as_deref(),
                service.service_principal_issuer.as_deref()
            );
        }
    }
}
