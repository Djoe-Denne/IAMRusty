//! QA coverage90 C: free text budgets and forbidden fields at nested boundaries.
use apparatus_contracts::{
    validate_install_ref, validate_manifest, validate_manifest_toml, ApparatusError,
    ApparatusManifest, MAX_ID_LEN, MAX_INSTALL_REF_LEN,
};

const MANIFEST: &str = r#"
[apparatus]
id = "io.aiforall.reference-kv"
version = "0.1.0"
schema_version = 1
[backend]
protocol = "manifesto-apparatus/1"
[capabilities]
requires = ["project.read"]
[ui]
mode = "absent"
"#;

fn manifest() -> ApparatusManifest {
    validate_manifest_toml(MANIFEST)
        .expect("minimal existing P0 schema")
        .manifest
}

#[test]
fn display_name_and_storage_have_independent_character_budgets() {
    for storage in [false, true] {
        let mut input = manifest();
        if storage {
            input.backend.storage = Some("界".repeat(MAX_ID_LEN));
        } else {
            input.apparatus.name = Some("界".repeat(MAX_ID_LEN));
        }
        assert_eq!(validate_manifest(&input), Ok(()));
        if storage {
            input.backend.storage.as_mut().expect("storage").push('界');
        } else {
            input.apparatus.name.as_mut().expect("name").push('界');
        }
        assert_eq!(
            validate_manifest(&input),
            Err(ApparatusError::IdTooLong {
                max: MAX_ID_LEN,
                actual: MAX_ID_LEN + 1
            })
        );
    }
}

#[test]
fn install_reference_over_budget_is_rejected_without_echoing_unbounded_input() {
    for input in [
        "x".repeat(MAX_INSTALL_REF_LEN + 1),
        "界".repeat(MAX_INSTALL_REF_LEN + 1),
    ] {
        let error = validate_install_ref(&input).expect_err("oversized ref");
        assert!(matches!(error, ApparatusError::InvalidDigest { .. }));
        assert!(!error.to_string().contains(&input));
        assert!(error.to_string().len() < 100);
    }
}

#[test]
fn forbidden_keys_inside_arrays_and_nested_tables_have_typed_refusal() {
    for key in [
        "Trusted_Override",
        "CREDENTIAL",
        "Password",
        "TOKEN",
        "JWT",
        "hmac_secret",
    ] {
        // Scan precedes schema parsing: hiding a privilege key in an unknown
        // extension must still yield the stable forbidden-field category.
        let input =
            format!("{MANIFEST}\n[[extension]]\n[extension.nested]\n{key} = 'fixture-only'\n");
        assert_eq!(
            validate_manifest_toml(&input),
            Err(ApparatusError::ForbiddenField { field: key.into() })
        );
    }
}

#[test]
fn unknown_manifest_fields_are_not_ignored_at_any_declared_section() {
    for section in ["apparatus", "backend", "capabilities", "ui"] {
        let input = MANIFEST.replacen(
            &format!("[{section}]"),
            &format!("[{section}]\nfuture_field = true"),
            1,
        );
        let error = validate_manifest_toml(&input).expect_err("closed schema");
        assert_eq!(error.code(), "APPARATUS_TOML_PARSE", "{section}: {error}");
    }
}
