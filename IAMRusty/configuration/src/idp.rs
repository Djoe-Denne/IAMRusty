//! Federated IdP connector registry (`[idp]`).

use serde::{Deserialize, Deserializer, Serialize};

/// Minimum HMAC secret length (bytes after trim). Same fail-closed rule as `HttpIdpConnector`.
const MIN_HMAC_SECRET_LEN: usize = 16;

/// Login/link callback vs relink callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdpRedirectFlow {
    /// `/callback` (login and link).
    Callback,
    /// `/relink-callback`.
    Relink,
}

/// One connector entry from `[[idp.connectors]]`.
#[derive(Clone, Serialize, Deserialize)]
pub struct IdpConnectorConfig {
    /// Provider slug (`github`, `gitlab`, …).
    pub id: String,
    /// Connector base URL including service prefix (`http://127.0.0.1:3000/github-connect`).
    pub base_url: String,
    /// HMAC secret. Never log this field.
    pub hmac_secret: String,
    /// Registered redirect URIs (callback and relink-callback).
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

impl std::fmt::Debug for IdpConnectorConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdpConnectorConfig")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("hmac_secret", &"***")
            .field("redirect_uris", &self.redirect_uris)
            .finish()
    }
}

impl IdpConnectorConfig {
    /// Pick the allowlisted redirect URI whose last path segment matches `flow`.
    ///
    /// Invalid URLs are skipped. The original allowlisted string is returned.
    #[must_use]
    pub fn redirect_uri(&self, flow: IdpRedirectFlow) -> Option<&str> {
        let wanted = match flow {
            IdpRedirectFlow::Relink => "relink-callback",
            IdpRedirectFlow::Callback => "callback",
        };
        self.redirect_uris.iter().find_map(|uri| {
            (last_path_segment(uri).as_deref() == Some(wanted)).then_some(uri.as_str())
        })
    }
}

/// `[idp]` section. Missing section deserializes as empty connectors.
///
/// Empty `Default` is valid for unit structs. Boot (`validate` / setup) rejects empty
/// registries and HMAC secrets shorter than 16 bytes.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IdpConfig {
    /// Per-slug connector registry.
    pub connectors: Vec<IdpConnectorConfig>,
}

impl<'de> Deserialize<'de> for IdpConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct RawIdpConfig {
            #[serde(default)]
            connectors: Vec<IdpConnectorConfig>,
        }
        let raw = RawIdpConfig::deserialize(deserializer)?;
        let config = Self {
            connectors: raw.connectors,
        };
        config
            .ensure_unique_connector_ids()
            .map_err(serde::de::Error::custom)?;
        Ok(config)
    }
}

impl IdpConfig {
    /// Reject duplicate connector ids (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns a message when two connectors share the same `id`.
    pub fn ensure_unique_connector_ids(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for connector in &self.connectors {
            let id = connector.id.to_ascii_lowercase();
            if !seen.insert(id) {
                return Err(format!("duplicate IdP connector id: {}", connector.id));
            }
        }
        Ok(())
    }

    /// Fail closed unless the registry has connectors and every HMAC secret is ≥16 bytes.
    ///
    /// Unknown slugs are allowed here; setup still fails if no known provider is wired.
    ///
    /// # Errors
    ///
    /// Returns a message when connectors are empty, ids collide, or a secret is too short.
    pub fn validate(&self) -> Result<(), String> {
        if self.connectors.is_empty() {
            return Err("idp.connectors must not be empty".to_string());
        }
        self.ensure_unique_connector_ids()?;
        for connector in &self.connectors {
            let trimmed = connector.hmac_secret.trim();
            if trimmed.is_empty() {
                return Err(format!(
                    "hmac_secret for IdP connector {} must not be empty",
                    connector.id
                ));
            }
            if trimmed.len() < MIN_HMAC_SECRET_LEN {
                return Err(format!(
                    "hmac_secret for IdP connector {} must be at least {MIN_HMAC_SECRET_LEN} bytes",
                    connector.id
                ));
            }
        }
        Ok(())
    }

    /// True when a connector id matches `slug` (case-insensitive).
    #[must_use]
    pub fn has_connector(&self, slug: &str) -> bool {
        self.connectors
            .iter()
            .any(|connector| connector.id.eq_ignore_ascii_case(slug))
    }

    /// Connector config for `slug`.
    #[must_use]
    pub fn connector(&self, slug: &str) -> Option<&IdpConnectorConfig> {
        self.connectors
            .iter()
            .find(|connector| connector.id.eq_ignore_ascii_case(slug))
    }

    /// Redirect URI for `slug` and `flow`.
    #[must_use]
    pub fn redirect_uri(&self, slug: &str, flow: IdpRedirectFlow) -> Option<&str> {
        self.connector(slug)?.redirect_uri(flow)
    }
}

/// Last path segment of `uri`, or `None` if the URI cannot be parsed.
fn last_path_segment(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    parsed
        .path_segments()?
        .next_back()
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connector(id: &str, hmac: &str) -> IdpConnectorConfig {
        IdpConnectorConfig {
            id: id.to_string(),
            base_url: "http://127.0.0.1:3000/github-connect".to_string(),
            hmac_secret: hmac.to_string(),
            redirect_uris: vec![
                "http://127.0.0.1:8081/iam/api/auth/github/callback".to_string(),
                "http://127.0.0.1:8081/iam/api/auth/github/relink-callback".to_string(),
            ],
        }
    }

    #[test]
    fn picks_callback_vs_relink() {
        let connector = connector("github", "iam-idp-connect-test-hmac");
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Callback),
            Some("http://127.0.0.1:8081/iam/api/auth/github/callback")
        );
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Relink),
            Some("http://127.0.0.1:8081/iam/api/auth/github/relink-callback")
        );
    }

    #[test]
    fn picks_callback_without_iam_prefix() {
        let connector = IdpConnectorConfig {
            id: "github".to_string(),
            base_url: String::new(),
            hmac_secret: "iam-idp-connect-test-hmac".to_string(),
            redirect_uris: vec![
                "http://127.0.0.1:8081/api/auth/github/callback".to_string(),
                "http://127.0.0.1:8081/api/auth/github/relink-callback".to_string(),
            ],
        };
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Callback),
            Some("http://127.0.0.1:8081/api/auth/github/callback")
        );
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Relink),
            Some("http://127.0.0.1:8081/api/auth/github/relink-callback")
        );
    }

    #[test]
    fn ignores_callback_substring_decoys() {
        let real_callback = "http://127.0.0.1:8081/iam/api/auth/github/callback";
        let real_relink = "http://127.0.0.1:8081/iam/api/auth/github/relink-callback";
        let connector = IdpConnectorConfig {
            id: "github".to_string(),
            base_url: "http://127.0.0.1:3000/github-connect".to_string(),
            hmac_secret: "iam-idp-connect-test-hmac".to_string(),
            redirect_uris: vec![
                "http://evil.example/?next=/callback".to_string(),
                "http://evil.example/callback.attacker".to_string(),
                real_relink.to_string(),
                real_callback.to_string(),
            ],
        };
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Callback),
            Some(real_callback)
        );
        assert_eq!(
            connector.redirect_uri(IdpRedirectFlow::Relink),
            Some(real_relink)
        );
        assert_ne!(
            connector.redirect_uri(IdpRedirectFlow::Callback),
            connector.redirect_uri(IdpRedirectFlow::Relink)
        );
    }

    #[test]
    fn debug_hides_hmac_secret() {
        let connector = connector("github", "super-secret");
        let debug = format!("{connector:?}");
        assert!(!debug.contains("super-secret"));
        assert!(debug.contains("***"));
    }

    #[test]
    fn deserialize_rejects_duplicate_connector_ids() {
        let json = r#"{
            "connectors": [
                {"id": "github", "base_url": "http://a", "hmac_secret": "one"},
                {"id": "GitHub", "base_url": "http://b", "hmac_secret": "two"}
            ]
        }"#;
        let err = serde_json::from_str::<IdpConfig>(json).expect_err("duplicate id");
        assert!(err.to_string().contains("duplicate IdP connector id"));
    }

    #[test]
    fn deserialize_accepts_unique_connector_ids() {
        let json = r#"{
            "connectors": [
                {"id": "github", "base_url": "http://a", "hmac_secret": "one"},
                {"id": "gitlab", "base_url": "http://b", "hmac_secret": "two"}
            ]
        }"#;
        let config: IdpConfig = serde_json::from_str(json).expect("unique ids");
        assert_eq!(config.connectors.len(), 2);
    }

    #[test]
    fn default_empty_is_ok_for_unit_structs() {
        let config = IdpConfig::default();
        assert!(config.connectors.is_empty());
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_connectors() {
        let err = IdpConfig::default().validate().expect_err("empty registry");
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn validate_rejects_short_hmac() {
        let config = IdpConfig {
            connectors: vec![connector("github", "fifteen-bytes!!")],
        };
        let err = config.validate().expect_err("short hmac");
        assert!(err.contains("at least 16"));
        assert_eq!("fifteen-bytes!!".len(), 15);
    }

    #[test]
    fn validate_accepts_hmac_of_16_bytes() {
        let config = IdpConfig {
            connectors: vec![connector("github", "sixteen-bytes-ok")],
        };
        config.validate().expect("16-byte hmac");
    }
}
