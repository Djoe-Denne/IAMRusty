//! SigningKey registry model (ADR-0304).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trust domain owning a signing key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustScope {
    Platform,
    Organization,
}

/// Lifecycle status for JWKS publication and signing eligibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningKeyStatus {
    /// Pre-published in JWKS before any signature (rotation N+1).
    Pending,
    /// Active signing + JWKS.
    Active,
    /// Still in JWKS for verification during rotation; do not mint new tokens.
    Retiring,
    /// Revoked; never in JWKS.
    Revoked,
}

impl SigningKeyStatus {
    /// Whether this key may appear in `GET /.well-known/jwks.json`.
    #[must_use]
    pub const fn in_jwks(&self) -> bool {
        matches!(self, Self::Pending | Self::Active | Self::Retiring)
    }

    /// Whether this key may mint new access tokens.
    #[must_use]
    pub const fn can_sign(&self) -> bool {
        matches!(self, Self::Active)
    }
}

impl From<&SigningKeyStatus> for String {
    fn from(status: &SigningKeyStatus) -> Self {
        match status {
            SigningKeyStatus::Pending => "pending".to_string(),
            SigningKeyStatus::Active => "active".to_string(),
            SigningKeyStatus::Retiring => "retiring".to_string(),
            SigningKeyStatus::Revoked => "revoked".to_string(),
        }
    }
}

impl std::str::FromStr for SigningKeyStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "retiring" => Ok(Self::Retiring),
            "revoked" => Ok(Self::Revoked),
            other => Err(format!("unknown signing key status: {other}")),
        }
    }
}

/// Provider backend type stored on a [`SigningKey`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningProviderType {
    PemFile,
    OpenBaoTransit,
    AwsKms,
    GcpKms,
    AzureKeyVault,
    /// HTTP Sign / GetPublicKey adapter (ADR-0309). Not a cloud BYOKMS.
    RemoteHttp,
}

impl SigningProviderType {
    /// Cloud BYOKMS types that IAM refuses to configure (no adapters yet).
    #[must_use]
    pub const fn is_unsupported_cloud_byokms(&self) -> bool {
        matches!(self, Self::AwsKms | Self::GcpKms | Self::AzureKeyVault)
    }
}

impl From<&SigningProviderType> for String {
    fn from(value: &SigningProviderType) -> Self {
        match value {
            SigningProviderType::PemFile => "pem_file".to_string(),
            SigningProviderType::OpenBaoTransit => "openbao_transit".to_string(),
            SigningProviderType::AwsKms => "aws_kms".to_string(),
            SigningProviderType::GcpKms => "gcp_kms".to_string(),
            SigningProviderType::AzureKeyVault => "azure_key_vault".to_string(),
            SigningProviderType::RemoteHttp => "remote_http".to_string(),
        }
    }
}

impl std::str::FromStr for SigningProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pem_file" => Ok(Self::PemFile),
            "openbao_transit" => Ok(Self::OpenBaoTransit),
            "aws_kms" => Ok(Self::AwsKms),
            "gcp_kms" => Ok(Self::GcpKms),
            "azure_key_vault" => Ok(Self::AzureKeyVault),
            "remote_http" | "remote" => Ok(Self::RemoteHttp),
            other => Err(format!("unknown signing provider type: {other}")),
        }
    }
}

/// Registry row for a platform or organization signing key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SigningKey {
    pub id: Uuid,
    /// Opaque public key id — never an org id, ARN, or OpenBao path.
    pub kid: String,
    pub algorithm: String,
    pub trust_scope: TrustScope,
    /// Issuer URL bound to this key (must match JWT `iss`).
    pub issuer: String,
    pub provider_type: SigningProviderType,
    pub provider_key_ref: String,
    pub credential_ref: Option<String>,
    pub public_key: String,
    pub status: SigningKeyStatus,
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Transit key name reserved for apparatus Cosign — never reuse for org JWT signing.
pub const FORBIDDEN_TRANSIT_KEY_NAME: &str = "apparatus-p4-cosign";

/// Access-token typ header value (ADR-0304 / rustycog-http).
pub const ACCESS_TOKEN_TYP: &str = "aiforall-access+jwt";

/// Clock skew added to access-token TTL when retiring keys leave the JWKS window.
pub const JWKS_RETIRE_SKEW_SECONDS: i64 = 60;

/// Opaque public `kid` — UUID v4 simple hex, never `org-` / ARN / vault path / slug.
#[must_use]
pub fn opaque_kid() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Drop retiring keys whose verification window (`updated_at` + access TTL + skew) has elapsed.
#[must_use]
pub fn filter_jwks_publication_keys(
    keys: Vec<SigningKey>,
    expiration_seconds: u64,
) -> Vec<SigningKey> {
    let now = Utc::now();
    let retain = chrono::Duration::seconds(expiration_seconds as i64 + JWKS_RETIRE_SKEW_SECONDS);
    keys.into_iter()
        .filter(|key| match key.status {
            SigningKeyStatus::Pending | SigningKeyStatus::Active => true,
            SigningKeyStatus::Retiring => key.updated_at + retain >= now,
            SigningKeyStatus::Revoked => false,
        })
        .collect()
}
