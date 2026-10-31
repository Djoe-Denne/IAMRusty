#![allow(missing_docs)]
use github_connect_configuration::{load_config, GitHubConfig, SecretsMode};
use serial_test::serial;

#[derive(Clone, Default)]
struct Capture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Environment(Vec<(String, Option<std::ffi::OsString>)>);
impl Environment {
    fn set(&mut self, key: &str, value: &str) {
        if !self.0.iter().any(|(name, _)| name == key) {
            self.0.push((key.to_owned(), std::env::var_os(key)));
        }
        std::env::set_var(key, value);
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

#[test]
#[serial]
fn real_prefixed_loader_is_declarative_strict_and_never_falls_back() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let mut env = Environment(Vec::new());
    for (key, value) in [
        ("RUN_ENV", "test"),
        ("GITHUB_CONNECT_GITHUB__SECRETS_MODE", "openbao"),
        ("GITHUB_CONNECT_GITHUB__CLIENT_SECRET", ""),
        ("GITHUB_CONNECT_GITHUB__HMAC_SECRET", ""),
        ("GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__MOUNT", "secret"),
        (
            "GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__PATH",
            "github-connect/oauth",
        ),
        (
            "GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__FIELD",
            "client_secret",
        ),
        ("GITHUB_CONNECT_GITHUB__HMAC_SECRET_REF__MOUNT", "secret"),
        (
            "GITHUB_CONNECT_GITHUB__HMAC_SECRET_REF__PATH",
            "github-connect/iam",
        ),
        (
            "GITHUB_CONNECT_GITHUB__HMAC_SECRET_REF__FIELD",
            "hmac_secret",
        ),
        (
            "GITHUB_CONNECT_OPENBAO__ENDPOINT",
            "https://bao.example.invalid",
        ),
        ("GITHUB_CONNECT_OPENBAO__AUTH__METHOD", "token_file"),
        (
            "GITHUB_CONNECT_OPENBAO__AUTH__PATH",
            "deliberately-absent-file",
        ),
    ] {
        env.set(key, value);
    }
    let config = load_config().expect("declarative loading without acquiring credentials");
    assert!(config.github.secrets_mode == SecretsMode::Openbao);
    assert!(config.github.client_secret_ref.is_some());

    // Valid direct values cannot rescue a bad OpenBao declaration.
    env.set(
        "GITHUB_CONNECT_GITHUB__CLIENT_SECRET",
        "sensitive-client-sentinel",
    );
    env.set(
        "GITHUB_CONNECT_GITHUB__HMAC_SECRET",
        "sensitive-hmac-sentinel",
    );
    for (key, value) in [
        (
            "GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__MOUNT",
            "../sensitive-reference-sentinel",
        ),
        (
            "GITHUB_CONNECT_GITHUB__SECRETS_MODE",
            "sensitive-unknown-mode-sentinel",
        ),
    ] {
        env.set("GITHUB_CONNECT_GITHUB__CLIENT_SECRET_REF__MOUNT", "secret");
        env.set(key, value);
        let error = load_config().expect_err("strict error, not defaults");
        let diagnostic = format!("{error:?} {error}");
        assert!(diagnostic.contains("Invalid configuration"));
        assert!(!diagnostic.contains("sentinel"));
    }
    assert!(!String::from_utf8(
        capture
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    )
    .unwrap()
    .contains("sentinel"));
}

#[test]
fn direct_values_are_not_serialized_and_default_source_is_unusable() {
    let config = GitHubConfig {
        client_secret: "client-sentinel".into(),
        hmac_secret: "hmac-sentinel".into(),
        ..GitHubConfig::default()
    };
    let encoded = serde_json::to_string(&config).unwrap();
    assert!(!encoded.contains("client-sentinel"));
    assert!(!encoded.contains("hmac-sentinel"));
    assert!(config.secrets_mode == SecretsMode::Unconfigured);
}
