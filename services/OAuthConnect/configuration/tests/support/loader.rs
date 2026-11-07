//! Service-local process fixture; SDK loader remains the only loader (G-1).

use super::support::assert_redacted;
use oauth_connect_configuration::{load_config, AppConfig, ConfigError};
use serde_json::Value;
use std::{env, fs, process::Command};

/// Require success without dumping errors or the loaded secrets.
pub fn loader_ok(result: Result<AppConfig, ConfigError>) -> AppConfig {
    assert!(
        result.is_ok(),
        "G-1: real SDK loader refused a valid isolated source"
    );
    result.unwrap()
}

/// Require a redacted rejection at the real loader boundary.
pub fn loader_error(result: Result<AppConfig, ConfigError>) {
    assert!(
        result.is_err(),
        "G-1: real SDK loader admitted an invalid source"
    );
    let error = result.unwrap_err();
    let text = format!("{error:?} {error}");
    assert!(!text.is_empty(), "G-1: rejection must have a diagnostic");
    assert_redacted(&text);
}

/// Re-execute the same test with child-only env/CWD, never mutate runner env.
/// Source assembly still belongs solely to the SDK (RUN_ENV, TOML, dotenv).
pub fn isolated(
    test: &str,
    case: &str,
    value: &Value,
    overrides: &[(&str, &str)],
    check: impl FnOnce(Result<AppConfig, ConfigError>),
) {
    if let Ok(selected) = env::var("OC_TEST_CHILD") {
        if selected == case {
            check(load_config());
            println!("OC_LOADER_CASE_ASSERTED");
        }
        return;
    }
    let directory = tempfile::tempdir().expect("isolated loader directory");
    fs::create_dir(directory.path().join("config")).unwrap();
    fs::write(
        directory.path().join("config/test.toml"),
        toml::to_string(value).unwrap(),
    )
    .unwrap();
    // A different default source must not override RUN_ENV=test.
    fs::write(
        directory.path().join("config/development.toml"),
        "unknown = 'source-sentinel'\n",
    )
    .unwrap();
    // Stop dotenv's upward search at the fixture, including when parent has a .env.
    fs::write(
        directory.path().join(".env"),
        "OAUTH_CONNECT_LOGGING__LEVEL=warn\n",
    )
    .unwrap();
    let mut command = Command::new(env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .current_dir(directory.path())
        .env_clear();
    // Windows process/toolchain essentials only; no ambient provider/SDK config.
    for key in ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(key) {
            command.env(key, value);
        }
    }
    command.env("RUN_ENV", "test").env("OC_TEST_CHILD", case);
    for (key, value) in overrides {
        command.env(key, value);
    }
    let output = command.output().expect("launch isolated real loader test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_redacted(&stdout);
    assert_redacted(&String::from_utf8_lossy(&output.stderr));
    assert!(
        output.status.success(),
        "G-1: isolated loader assertion failed in {test}/{case}"
    );
    assert!(
        stdout.contains("running 1 test"),
        "G-1: child must discover exactly one real test"
    );
    assert!(
        stdout.contains("OC_LOADER_CASE_ASSERTED"),
        "G-1: child must execute the selected oracle"
    );
    // TempDir removes the source and .env only after the child has exited.
}
