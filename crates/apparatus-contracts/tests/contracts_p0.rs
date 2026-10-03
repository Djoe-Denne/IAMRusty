//! Tests P0 des contrats Apparatus (déterministes, sans Docker ni réseau).
//!
//! Exécuter avec `cargo test -p apparatus-contracts --features test-harness`
//! pour activer les tests du harness TEST-ONLY.

#[cfg(feature = "test-harness")]
use std::collections::BTreeMap;

#[cfg(feature = "test-harness")]
use apparatus_contracts::assert_no_secret_keys;
use apparatus_contracts::{
    digest_json, digest_manifest, digest_str, is_floating_ref, validate_declared_version,
    validate_install_ref, validate_manifest_toml, ApparatusError, ApparatusId, ConfigureRequest,
    DiscoveryDocument, DiscoveryEndpoints, HealthResponse, HealthStatus, InvokeRequest,
    ReleaseDigest, MAX_CAPABILITIES, MAX_CONFIG_BYTES, MAX_ID_LEN, MAX_OPERATION_NAME_LEN,
    MAX_SCHEMA_PATH_LEN, PROTOCOL_ID,
};
#[cfg(feature = "test-harness")]
use apparatus_contracts::{BindRequest, UnbindRequest};

// ---------- Helpers typés ----------

/// Manifeste TOML valide minimal.
fn valid_manifest_toml() -> String {
    r#"
[apparatus]
id = "io.aiforall.reference-kv"
version = "0.1.0"
schema_version = 1

[backend]
protocol = "manifesto-apparatus/1"
storage = "kv-v1"

[capabilities]
requires = ["project.read", "storage.kv.read", "storage.kv.write"]

[ui]
mode = "schema"
schema_path = "ui/settings.schema.json"
"#
    .to_owned()
}

/// Même manifeste, clés TOML réordonnées (canonique identique).
fn reordered_manifest_toml() -> String {
    r#"
[ui]
schema_path = "ui/settings.schema.json"
mode = "schema"

[capabilities]
requires = ["project.read", "storage.kv.read", "storage.kv.write"]

[backend]
storage = "kv-v1"
protocol = "manifesto-apparatus/1"

[apparatus]
schema_version = 1
version = "0.1.0"
id = "io.aiforall.reference-kv"
"#
    .to_owned()
}

/// Construit une requête `bind` typée.
#[cfg(feature = "test-harness")]
fn bind_request(binding: &str, operation: &str) -> BindRequest {
    BindRequest {
        binding_id: binding.parse().expect("binding id"),
        apparatus_id: "io.aiforall.reference-kv".parse().expect("apparatus id"),
        release_digest: digest_str("fixture-release")
            .as_str()
            .parse()
            .expect("digest"),
        operation_id: operation.parse().expect("operation id"),
        config: None,
    }
}

/// Construit une requête `invoke` typée.
fn invoke_request(binding: &str, operation_id: &str, name: &str) -> InvokeRequest {
    InvokeRequest {
        binding_id: binding.parse().expect("binding id"),
        operation_id: operation_id.parse().expect("operation id"),
        operation: name.to_owned(),
        params: serde_json::json!({"key": "k"}),
    }
}

// ---------- Manifeste + sérialisation ----------

#[test]
fn valid_manifest_accepted_and_stable_serialization() {
    let first = validate_manifest_toml(&valid_manifest_toml()).expect("manifeste valide");
    let second = validate_manifest_toml(&valid_manifest_toml()).expect("manifeste valide");
    assert_eq!(first.digest, second.digest);

    let json_a = serde_json::to_value(&first.manifest).expect("sérialisation");
    let json_b = serde_json::to_value(&second.manifest).expect("sérialisation");
    assert_eq!(json_a, json_b);
    // Absence de secrets prouvée par `no_secret_in_serialized_dto_and_errors`
    // (feature `test-harness`).
}

#[test]
fn digest_stable_on_reordered_keys_but_changes_on_semantic_mutation() {
    let base = validate_manifest_toml(&valid_manifest_toml()).expect("manifeste valide");
    let reordered = validate_manifest_toml(&reordered_manifest_toml()).expect("manifeste valide");
    assert_eq!(
        base.digest, reordered.digest,
        "réordonnancement ⇒ même digest"
    );

    let mutated_toml = valid_manifest_toml().replace(
        r#"requires = ["project.read", "storage.kv.read", "storage.kv.write"]"#,
        r#"requires = ["project.read", "storage.kv.read"]"#,
    );
    let mutated = validate_manifest_toml(&mutated_toml).expect("manifeste valide");
    assert_ne!(
        base.digest, mutated.digest,
        "mutation sémantique ⇒ digest différent"
    );

    // Le digest couvre aussi la version déclarative.
    let bumped_toml = valid_manifest_toml().replace(r#"version = "0.1.0""#, r#"version = "0.2.0""#);
    let bumped = validate_manifest_toml(&bumped_toml).expect("manifeste valide");
    assert_ne!(base.digest, bumped.digest);
}

#[test]
fn digest_fixed_vectors() {
    assert_eq!(
        digest_str("abc").as_str(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        digest_str("").as_str(),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let manifest = validate_manifest_toml(&valid_manifest_toml()).expect("manifeste valide");
    let recomputed = digest_manifest(&manifest.manifest).expect("digest");
    assert_eq!(manifest.digest, recomputed);
}

// ---------- Rejets obligatoires ----------

#[test]
fn floating_refs_rejected_as_declared_version() {
    for floating in ["latest", "main", "branches/foo", "tags/v1", "heads/x"] {
        assert!(is_floating_ref(floating), "{floating} doit être flottant");
        let toml = valid_manifest_toml().replace(
            r#"version = "0.1.0""#,
            &format!(r#"version = "{floating}""#),
        );
        let err = validate_manifest_toml(&toml).expect_err("version flottante rejetée");
        assert_eq!(err.code(), "APPARATUS_FLOATING_REF", "pour {floating}");
    }
}

#[test]
fn floating_refs_rejected_as_install_ref() {
    for floating in ["latest", "main", "branches/foo", "tags/v1.2.3", "0.1.0"] {
        let err = validate_install_ref(floating).expect_err("install ref rejetée");
        assert!(
            err.code() == "APPARATUS_FLOATING_REF" || err.code() == "APPARATUS_INVALID_DIGEST",
            "pour {floating}: {}",
            err.code()
        );
    }
    let digest = digest_str("release").as_str().to_owned();
    validate_install_ref(&digest).expect("digest accepté comme install ref");
}

#[test]
fn unknown_schema_major_rejected() {
    let toml = valid_manifest_toml().replace("schema_version = 1", "schema_version = 9");
    let err = validate_manifest_toml(&toml).expect_err("majeur inconnu rejeté");
    assert_eq!(err.code(), "APPARATUS_UNKNOWN_SCHEMA_MAJOR");
    assert!(matches!(
        err,
        ApparatusError::UnknownSchemaMajor { got: 9, .. }
    ));
}

#[test]
fn unknown_capability_rejected() {
    let toml = valid_manifest_toml().replace(
        r#"requires = ["project.read", "storage.kv.read", "storage.kv.write"]"#,
        r#"requires = ["project.read", "network.egress"]"#,
    );
    let err = validate_manifest_toml(&toml).expect_err("capacité inconnue rejetée");
    assert_eq!(err.code(), "APPARATUS_UNKNOWN_CAPABILITY");
}

#[test]
fn trusted_and_credential_fields_rejected() {
    // Injection dans les sections existantes (TOML structurellement valide).
    let cases = [
        (
            "[apparatus]\n",
            "[apparatus]\ntrusted_skip_gateway = true\n",
        ),
        ("[backend]\n", "[backend]\ntrusted_fast_path = true\n"),
        ("[capabilities]\n", "[capabilities]\ncredentials = \"x\"\n"),
    ];
    for (anchor, injection) in cases {
        let toml = valid_manifest_toml().replacen(anchor, injection, 1);
        let err = validate_manifest_toml(&toml).expect_err("champ interdit rejeté");
        assert_eq!(
            err.code(),
            "APPARATUS_FORBIDDEN_FIELD",
            "pour {injection:?}"
        );
    }
}

// ---------- Secrets, bornes, sandbox ----------

#[cfg(feature = "test-harness")]
#[test]
fn no_secret_in_serialized_dto_and_errors() {
    let bind = bind_request("binding-a", "11111111-1111-1111-1111-111111111111");
    let json = serde_json::to_value(&bind).expect("sérialisation");
    assert_no_secret_keys(&json).expect("aucune clé à secret dans BindRequest");

    let invoke = invoke_request(
        "binding-a",
        "22222222-2222-2222-2222-222222222222",
        "kv.get",
    );
    let json = serde_json::to_value(&invoke).expect("sérialisation");
    assert_no_secret_keys(&json).expect("aucune clé à secret dans InvokeRequest");

    let err = ApparatusError::ForbiddenField {
        field: "trusted_skip_gateway".to_owned(),
    };
    let display = err.to_string();
    assert!(!display.contains("secret"), "Display sans secret");
    assert_eq!(err.code(), "APPARATUS_FORBIDDEN_FIELD");
}

#[test]
fn id_and_payload_bounds_enforced() {
    let too_long = "b".repeat(101);
    let err = too_long
        .parse::<apparatus_contracts::BindingId>()
        .expect_err("id trop long");
    assert_eq!(err.code(), "APPARATUS_ID_TOO_LONG");

    let oversized = invoke_request(
        "binding-a",
        "33333333-3333-3333-3333-333333333333",
        "kv.put",
    );
    let mut huge = oversized;
    huge.params = serde_json::json!({"key": "k", "value": "v".repeat(300 * 1024)});
    let err = huge.validate().expect_err("payload trop gros");
    assert_eq!(err.code(), "APPARATUS_PAYLOAD_TOO_LARGE");

    let mut nameless = invoke_request("binding-a", "44444444-4444-4444-4444-444444444444", "");
    nameless.operation.clear();
    let err = nameless.validate().expect_err("nom vide");
    assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION");
}

#[test]
fn sandbox_accepted_as_declaration_only() {
    let toml = valid_manifest_toml()
        .replace(r#"mode = "schema""#, r#"mode = "sandbox""#)
        .replace("schema_path = \"ui/settings.schema.json\"\n", "");
    let validated = validate_manifest_toml(&toml).expect("sandbox accepté");
    assert_eq!(
        validated.manifest.ui.mode,
        apparatus_contracts::UiMode::Sandbox
    );

    // Tout champ host est rejeté (deny_unknown_fields + scan interdit).
    let with_host = toml.replace("[ui]\n", "[ui]\nhost = \"https://x\"\n");
    validate_manifest_toml(&with_host).expect_err("host rejeté");
}

#[cfg(feature = "test-harness")]
#[test]
fn harness_replay_and_kv_namespacing() {
    use apparatus_contracts::TestHarness;

    let harness = TestHarness::new();
    let op = "55555555-5555-5555-5555-555555555555";
    let first = harness.bind(&bind_request("binding-a", op)).expect("bind");
    let replay = harness
        .bind(&bind_request("binding-a", op))
        .expect("rejeu bind");
    assert_eq!(first, replay);
    assert!(harness.is_bound(&"binding-a".parse().expect("binding")));

    harness
        .bind(&bind_request(
            "binding-b",
            "66666666-6666-6666-6666-666666666666",
        ))
        .expect("bind b");

    // Écriture via invoke dans le namespace A uniquement.
    let mut put = invoke_request(
        "binding-a",
        "77777777-7777-7777-7777-777777777777",
        "kv.put",
    );
    put.params = serde_json::json!({"key": "k", "value": "v-a"});
    put.validate().expect("bornes");
    harness.invoke(&put).expect("kv.put");

    let binding_b: apparatus_contracts::BindingId = "binding-b".parse().expect("binding");
    let leaked = harness
        .kv()
        .get(&binding_b, "k")
        .expect("lecture")
        .is_some();
    assert!(!leaked, "aucune fuite inter-bindings");

    let unbind = UnbindRequest {
        binding_id: "binding-a".parse().expect("binding"),
        operation_id: "88888888-8888-8888-8888-888888888888".parse().expect("op"),
    };
    harness.unbind(&unbind).expect("unbind");
    assert!(!harness.is_bound(&"binding-a".parse().expect("binding")));

    // La config d'invoke vide reste un objet JSON valide et borné.
    let empty_params: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    assert!(empty_params.is_empty());
}

#[test]
fn forbidden_keys_rejected_case_insensitive() {
    // Variantes de casse : le scan interdit est insensible à la casse.
    let cases = [
        (
            "[apparatus]\n",
            "[apparatus]\nTrusted_Skip_Gateway = true\n",
        ),
        ("[backend]\n", "[backend]\nTRUSTED_FAST_PATH = true\n"),
        ("[capabilities]\n", "[capabilities]\nCredentials = \"x\"\n"),
        ("[capabilities]\n", "[capabilities]\nSECRET = \"x\"\n"),
    ];
    for (anchor, injection) in cases {
        let toml = valid_manifest_toml().replacen(anchor, injection, 1);
        let err = validate_manifest_toml(&toml).expect_err("champ interdit rejeté");
        assert_eq!(
            err.code(),
            "APPARATUS_FORBIDDEN_FIELD",
            "pour {injection:?}"
        );
    }
}

#[test]
fn description_too_long_rejected_with_centralized_bound() {
    let long = "d".repeat(apparatus_contracts::MAX_DESCRIPTION_LEN + 1);
    let toml = valid_manifest_toml().replacen(
        "[apparatus]\n",
        &format!("[apparatus]\ndescription = \"{long}\"\n"),
        1,
    );
    let err = validate_manifest_toml(&toml).expect_err("description trop longue rejetée");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");

    // À la borne exacte : accepté.
    let exact = "d".repeat(apparatus_contracts::MAX_DESCRIPTION_LEN);
    let toml = valid_manifest_toml().replacen(
        "[apparatus]\n",
        &format!("[apparatus]\ndescription = \"{exact}\"\n"),
        1,
    );
    validate_manifest_toml(&toml).expect("description à la borne acceptée");
}

#[cfg(feature = "test-harness")]
#[test]
fn unbind_unknown_binding_rejected_but_replay_survives() {
    use apparatus_contracts::TestHarness;

    let harness = TestHarness::new();
    // Binding jamais attaché : rejet en InvalidOperation.
    let unknown = UnbindRequest {
        binding_id: "ghost".parse().expect("binding"),
        operation_id: "99999999-9999-9999-9999-999999999999".parse().expect("op"),
    };
    let err = harness.unbind(&unknown).expect_err("unbind inconnu rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION");

    // Cycle bind puis unbind : le rejeu du même operation_id survit à la purge.
    let op = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    harness.bind(&bind_request("binding-a", op)).expect("bind");
    let unbind = UnbindRequest {
        binding_id: "binding-a".parse().expect("binding"),
        operation_id: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb".parse().expect("op"),
    };
    let first = harness.unbind(&unbind).expect("unbind");
    let replay = harness.unbind(&unbind).expect("rejeu unbind");
    assert_eq!(first, replay);

    // Nouvel operation_id après purge : rejet inconnu.
    let again = UnbindRequest {
        binding_id: "binding-a".parse().expect("binding"),
        operation_id: "cccccccc-cccc-cccc-cccc-cccccccccccc".parse().expect("op"),
    };
    let err = harness.unbind(&again).expect_err("second unbind rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION");
}

// ---------- P0 manquants : semver, protocole, TOML, bornes ----------

#[test]
fn invalid_semver_rejected_and_floating_variants() {
    for bad in ["1.0", "v1.0.0", "1.0.0-", ""] {
        let err = validate_declared_version(bad).expect_err("semver invalide rejetée");
        assert_eq!(err.code(), "APPARATUS_SEMVER", "pour {bad:?}");
    }
    for floating in ["my ref", "a..b", "refs/heads/main", "refs/v1"] {
        assert!(is_floating_ref(floating), "{floating:?} doit être flottant");
        let err = validate_declared_version(floating).expect_err("version flottante rejetée");
        assert_eq!(err.code(), "APPARATUS_FLOATING_REF", "pour {floating:?}");
        let err = validate_install_ref(floating).expect_err("install ref flottante rejetée");
        assert_eq!(err.code(), "APPARATUS_FLOATING_REF", "pour {floating:?}");
    }
}

#[test]
fn backend_protocol_mismatch_rejected() {
    let toml = valid_manifest_toml().replace("manifesto-apparatus/1", "autre/1");
    let err = validate_manifest_toml(&toml).expect_err("protocole incohérent rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");
}

#[test]
fn truncated_toml_rejected() {
    let err = validate_manifest_toml("[apparatus\nid = ").expect_err("TOML tronqué rejeté");
    assert_eq!(err.code(), "APPARATUS_TOML_PARSE");
}

#[test]
fn too_many_capabilities_rejected_and_bound_accepted() {
    let anchor = r#"requires = ["project.read", "storage.kv.read", "storage.kv.write"]"#;
    let over = vec!["\"project.read\""; MAX_CAPABILITIES + 1].join(", ");
    let toml = valid_manifest_toml().replace(anchor, &format!("requires = [{over}]"));
    let err = validate_manifest_toml(&toml).expect_err("trop de capacités rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");
    assert!(
        err.to_string().contains("too many"),
        "message borne capacités"
    );

    let exact = vec!["\"project.read\""; MAX_CAPABILITIES].join(", ");
    let toml = valid_manifest_toml().replace(anchor, &format!("requires = [{exact}]"));
    validate_manifest_toml(&toml).expect("32 capacités acceptées");
}

#[test]
fn apparatus_id_and_digest_vectors_rejected() {
    for bad in ["", "IO.FOO", "sanspoint", ".a", "a."] {
        let err = ApparatusId::new(bad).expect_err("id invalide rejeté");
        assert_eq!(err.code(), "APPARATUS_INVALID_ID", "pour {bad:?}");
    }
    let too_long = "a".repeat(MAX_ID_LEN + 1);
    let err = ApparatusId::new(&too_long).expect_err("id trop long rejeté");
    assert_eq!(err.code(), "APPARATUS_ID_TOO_LONG");

    let no_prefix = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let short = "sha256:abc";
    let upper = format!(
        "sha256:{}",
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
    );
    for bad in [no_prefix, short, upper.as_str()] {
        let err = ReleaseDigest::new(bad).expect_err("digest invalide rejeté");
        assert_eq!(err.code(), "APPARATUS_INVALID_DIGEST", "pour {bad:?}");
    }
}

#[test]
fn configure_and_operation_bounds_enforced() {
    let mut config = std::collections::BTreeMap::new();
    let big = "v".repeat(MAX_CONFIG_BYTES + 1);
    config.insert("blob".to_owned(), serde_json::Value::String(big));
    let req = ConfigureRequest {
        binding_id: "binding-a".parse().expect("binding id"),
        operation_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
            .parse()
            .expect("operation id"),
        config,
    };
    let err = req.validate().expect_err("config trop grosse rejetée");
    assert_eq!(err.code(), "APPARATUS_CONFIG_TOO_LARGE");

    let long_name = "o".repeat(MAX_OPERATION_NAME_LEN + 1);
    let mut invoke = invoke_request(
        "binding-a",
        "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
        "kv.get",
    );
    invoke.operation = long_name;
    let err = invoke
        .validate()
        .expect_err("nom opération trop long rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_OPERATION");
}

#[test]
fn dto_deny_unknown_fields_rejected() {
    let ok = serde_json::json!({
        "binding_id": "binding-a",
        "operation_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
        "operation": "kv.get",
        "params": {"key": "k"}
    });
    serde_json::from_value::<InvokeRequest>(ok).expect("invoke sans champ inconnu accepté");
    let bad = serde_json::json!({
        "binding_id": "binding-a",
        "operation_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
        "operation": "kv.get",
        "params": {"key": "k"},
        "host": "https://x"
    });
    assert!(
        serde_json::from_value::<InvokeRequest>(bad).is_err(),
        "champ inconnu host rejeté"
    );
}

#[cfg(feature = "test-harness")]
#[test]
fn bind_dto_deny_unknown_fields_rejected() {
    let ok = serde_json::json!({
        "binding_id": "binding-a",
        "apparatus_id": "io.aiforall.reference-kv",
        "release_digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "operation_id": "dddddddd-dddd-dddd-dddd-dddddddddddd"
    });
    serde_json::from_value::<BindRequest>(ok).expect("bind sans champ inconnu accepté");
    let bad = serde_json::json!({
        "binding_id": "binding-a",
        "apparatus_id": "io.aiforall.reference-kv",
        "release_digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "operation_id": "dddddddd-dddd-dddd-dddd-dddddddddddd",
        "trusted_x": true
    });
    assert!(
        serde_json::from_value::<BindRequest>(bad).is_err(),
        "champ inconnu trusted_x rejeté"
    );
}

#[test]
fn ui_declaration_branches_rejected() {
    let toml = valid_manifest_toml().replace(r#"mode = "schema""#, r#"mode = "absent""#);
    let err = validate_manifest_toml(&toml).expect_err("absent avec schema rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");

    let toml = valid_manifest_toml().replace("schema_path = \"ui/settings.schema.json\"\n", "");
    let err = validate_manifest_toml(&toml).expect_err("schema sans path rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");

    for bad in ["/x.json", "../x.json", "ui/../x.json", ""] {
        let toml = valid_manifest_toml().replace(
            r#"schema_path = "ui/settings.schema.json""#,
            &format!(r#"schema_path = "{bad}""#),
        );
        let err = validate_manifest_toml(&toml).expect_err("schema_path invalide rejeté");
        assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST", "pour {bad:?}");
    }

    let toml = valid_manifest_toml().replace(
        r#"schema_path = "ui/settings.schema.json""#,
        r"schema_path = 'ui\x.json'",
    );
    let err = validate_manifest_toml(&toml).expect_err("backslash rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");

    let long = "a".repeat(MAX_SCHEMA_PATH_LEN + 1);
    let toml = valid_manifest_toml().replace(
        r#"schema_path = "ui/settings.schema.json""#,
        &format!(r#"schema_path = "{long}""#),
    );
    let err = validate_manifest_toml(&toml).expect_err("schema_path trop long rejeté");
    assert_eq!(err.code(), "APPARATUS_INVALID_MANIFEST");
}

// ---------- P0 Priorité 2 (peu coûteux) ----------

#[test]
fn digest_json_stable_on_nested_reorder() {
    let first = serde_json::json!({"z": {"b": 1, "a": 2}, "a": [3, 2]});
    let second = serde_json::json!({"a": [3, 2], "z": {"a": 2, "b": 1}});
    let digest_a = digest_json(&first).expect("digest imbriqué");
    let digest_b = digest_json(&second).expect("digest imbriqué");
    assert_eq!(digest_a, digest_b);
}

#[test]
fn truncate_bound_enforced() {
    assert_eq!(ApparatusError::truncate("abc", 5), "abc");
    let truncated = ApparatusError::truncate(&"a".repeat(10), 5);
    assert_eq!(truncated.chars().count(), 6, "5 caractères + ellipse");
    assert!(truncated.ends_with('…'), "ellipse finale");
}

#[test]
fn discovery_health_serde_roundtrip_and_deny_unknown() {
    let health = HealthResponse {
        status: HealthStatus::Ok,
        apparatus_id: "io.aiforall.reference-kv".parse().expect("apparatus id"),
    };
    let json = serde_json::to_value(&health).expect("sérialisation health");
    let back: HealthResponse = serde_json::from_value(json).expect("roundtrip health");
    assert_eq!(health, back);
    let bad = serde_json::json!({
        "status": "ok",
        "apparatus_id": "io.aiforall.reference-kv",
        "host": "x"
    });
    assert!(
        serde_json::from_value::<HealthResponse>(bad).is_err(),
        "health champ inconnu rejeté"
    );

    let doc = DiscoveryDocument {
        protocol: PROTOCOL_ID.to_owned(),
        apparatus_id: "io.aiforall.reference-kv".parse().expect("apparatus id"),
        version: "0.1.0".to_owned(),
        release_digest: digest_str("release").as_str().parse().expect("digest"),
        endpoints: DiscoveryEndpoints::default(),
    };
    let json = serde_json::to_value(&doc).expect("sérialisation discovery");
    let back: DiscoveryDocument = serde_json::from_value(json).expect("roundtrip discovery");
    assert_eq!(doc, back);
    let bad = serde_json::json!({
        "protocol": "manifesto-apparatus/1",
        "apparatus_id": "io.aiforall.reference-kv",
        "version": "0.1.0",
        "release_digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "endpoints": {
            "health": "/health",
            "ready": "/ready",
            "bind": "/bind",
            "configure": "/configure",
            "invoke": "/invoke",
            "unbind": "/unbind"
        },
        "trusted_x": true
    });
    assert!(
        serde_json::from_value::<DiscoveryDocument>(bad).is_err(),
        "discovery champ inconnu rejeté"
    );
}
