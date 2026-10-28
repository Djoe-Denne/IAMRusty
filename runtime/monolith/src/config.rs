use rustycog::config::ServerConfig;

pub struct MonolithConfig {
    pub server: ServerConfig,
    pub iam: iam_configuration::AppConfig,
    pub telegraph: telegraph_configuration::TelegraphConfig,
    pub hive: hive_configuration::AppConfig,
    pub manifesto: manifesto_configuration::AppConfig,
    pub lazaret: lazaret_configuration::AppConfig,
}

/// Overlay TLS for `rustycog` dual-bind (HTTP 8080 + HTTPS `tls_port`).
/// Default off so host J1 (`just monolith`) stays plain HTTP.
///
/// When enabled (J3 kind overlay), Lazaret nest `ensure_boot_tls` and identity CA
/// paths are aligned to the same files rustycog uses as `tls_client_ca_path`.
///
/// # Errors
///
/// Returns an error if any nested service config cannot be loaded.
pub fn load_monolith_config() -> anyhow::Result<MonolithConfig> {
    let iam = iam_configuration::load_config()?;
    let telegraph = telegraph_configuration::load_config()?;
    let hive = hive_configuration::load_config()?;
    let manifesto = manifesto_configuration::load_config()?;
    let mut lazaret = lazaret_configuration::load_config()?;
    validate_principal_policies(
        &iam.jwt.platform_issuer(),
        &[
            &hive.auth.jwt,
            &telegraph.auth.jwt,
            &manifesto.auth.jwt,
            &lazaret.auth.jwt,
        ],
    )?;

    let server = match overlay_tls_from_env() {
        None => ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls_enabled: false,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            tls_client_ca_path: String::new(),
            tls_require_client_cert: false,
            tls_port: 0,
        },
        Some(tls) => {
            // Nest Lazaret: generate-if-absent leaf + write trust anchor (T13).
            lazaret.server.tls_enabled = true;
            lazaret.server.tls_cert_path.clone_from(&tls.cert_path);
            lazaret.server.tls_key_path.clone_from(&tls.key_path);
            lazaret
                .server
                .tls_client_ca_path
                .clone_from(&tls.client_ca_path);
            lazaret.server.tls_port = tls.port;
            // Same CA instance for enroll signing and rustycog client CA.
            lazaret
                .identity
                .ca_cert_pem_path
                .clone_from(&tls.client_ca_path);
            lazaret
                .identity
                .ca_key_pem_path
                .clone_from(&tls.ca_key_path);

            ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                tls_enabled: true,
                tls_cert_path: tls.cert_path,
                tls_key_path: tls.key_path,
                tls_client_ca_path: tls.client_ca_path,
                tls_require_client_cert: false,
                tls_port: tls.port,
            }
        }
    };

    Ok(MonolithConfig {
        server,
        iam,
        telegraph,
        hive,
        manifesto,
        lazaret,
    })
}

fn validate_principal_policies(
    expected_platform_issuer: &str,
    configs: &[&rustycog::config::JwtAuthConfig],
) -> anyhow::Result<()> {
    let mut historical_window = None;
    for config in configs {
        let issuer = config
            .platform_issuer
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("PRINCIPAL_POLICY_UNAVAILABLE"))?;
        let historical = config
            .allow_historical_platform_issuer
            .ok_or_else(|| anyhow::anyhow!("PRINCIPAL_POLICY_UNAVAILABLE"))?;
        rustycog::permission::PrincipalPolicy::new(issuer, historical)?;
        if issuer != expected_platform_issuer
            || historical_window.is_some_and(|previous| previous != historical)
        {
            anyhow::bail!("PRINCIPAL_INVALID");
        }
        historical_window = Some(historical);
    }
    Ok(())
}

#[cfg(test)]
mod principal_policy_tests {
    use super::*;

    #[test]
    fn monolith_principal_policy_is_explicit_and_consistent() {
        let configured = rustycog::config::JwtAuthConfig {
            platform_issuer: Some("https://platform.example/iam".into()),
            allow_historical_platform_issuer: Some(false),
            ..Default::default()
        };
        assert!(validate_principal_policies(
            "https://platform.example/iam",
            &[&configured, &configured]
        )
        .is_ok());
        let mut foreign = configured.clone();
        foreign.platform_issuer = Some("https://independent.example/iam".into());
        assert!(validate_principal_policies(
            "https://platform.example/iam",
            &[&configured, &foreign]
        )
        .is_err());
        foreign = configured.clone();
        foreign.allow_historical_platform_issuer = Some(true);
        assert!(validate_principal_policies(
            "https://platform.example/iam",
            &[&configured, &foreign]
        )
        .is_err());
        assert!(validate_principal_policies(
            "https://platform.example/iam",
            &[&Default::default()]
        )
        .is_err());
    }
}

struct OverlayTls {
    port: u16,
    cert_path: String,
    key_path: String,
    client_ca_path: String,
    ca_key_path: String,
}

fn overlay_tls_from_env() -> Option<OverlayTls> {
    if !env_truthy("OODHIVE_TLS_ENABLED") {
        return None;
    }
    let port = std::env::var("OODHIVE_TLS_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8443);
    let cert_path = env_or("OODHIVE_TLS_CERT_PATH", "/app/certs/server.crt");
    let key_path = env_or("OODHIVE_TLS_KEY_PATH", "/app/certs/server.key");
    let client_ca_path = env_or("OODHIVE_TLS_CLIENT_CA_PATH", "/app/certs/ca.crt");
    let ca_key_path = env_or("OODHIVE_TLS_CA_KEY_PATH", "/app/certs/ca.key");
    Some(OverlayTls {
        port,
        cert_path,
        key_path,
        client_ca_path,
        ca_key_path,
    })
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_truthy(key: &str) -> bool {
    matches!(
        std::env::var(key).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}
