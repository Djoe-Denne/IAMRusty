//! Tests P0 de l'Apparatus KV de référence (déterministes, sans Docker ni réseau).

use std::collections::BTreeMap;

use apparatus_contracts::{
    validate_manifest_toml, ApparatusError, BindRequest, ConfigureRequest, InvokeRequest,
    UnbindRequest, MAX_KV_KEY_LEN, MAX_KV_VALUE_BYTES,
};
use apparatus_reference_kv::{
    deterministic_operation_id, InMemoryKvStore, ReferenceKvBackend, ReferenceKvError,
    REFERENCE_APPARATUS_ID,
};

// ---------- Helpers typés ----------

/// Backend de test avec stockage injecté.
fn backend() -> ReferenceKvBackend<InMemoryKvStore> {
    ReferenceKvBackend::new(InMemoryKvStore::new())
}

/// Requête `bind` typée et déterministe.
fn bind_request(binding: &str, op_label: &str) -> BindRequest {
    BindRequest {
        binding_id: binding.parse().expect("binding id"),
        apparatus_id: REFERENCE_APPARATUS_ID.parse().expect("apparatus id"),
        release_digest: apparatus_contracts::digest_str("reference-release")
            .as_str()
            .parse()
            .expect("digest"),
        operation_id: deterministic_operation_id(op_label),
        config: None,
    }
}

/// Requête `configure` typée.
fn configure_request(binding: &str, op_label: &str) -> ConfigureRequest {
    let mut config = BTreeMap::new();
    config.insert("page_size".to_owned(), serde_json::json!(20));
    ConfigureRequest {
        binding_id: binding.parse().expect("binding id"),
        operation_id: deterministic_operation_id(op_label),
        config,
    }
}

/// Requête `invoke` typée (`kv.put` / `kv.get` / `kv.delete`).
fn invoke_request(
    binding: &str,
    op_label: &str,
    name: &str,
    key: &str,
    value: Option<&str>,
) -> InvokeRequest {
    let mut params = serde_json::json!({"key": key});
    if let Some(text) = value {
        params["value"] = serde_json::json!(text);
    }
    InvokeRequest {
        binding_id: binding.parse().expect("binding id"),
        operation_id: deterministic_operation_id(op_label),
        operation: name.to_owned(),
        params,
    }
}

/// Requête `unbind` typée.
fn unbind_request(binding: &str, op_label: &str) -> UnbindRequest {
    UnbindRequest {
        binding_id: binding.parse().expect("binding id"),
        operation_id: deterministic_operation_id(op_label),
    }
}

// ---------- Idempotence ----------

#[test]
fn bind_configure_unbind_idempotent_on_operation_replay() {
    let service = backend();

    let first_bind = service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");
    let replay_bind = service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("rejeu");
    assert_eq!(first_bind, replay_bind);

    let first_cfg = service
        .configure(&configure_request("binding-a", "cfg-a"))
        .expect("configure");
    let replay_cfg = service
        .configure(&configure_request("binding-a", "cfg-a"))
        .expect("rejeu");
    assert_eq!(first_cfg, replay_cfg);

    let first_unbind = service
        .unbind(&unbind_request("binding-a", "unbind-a"))
        .expect("unbind");
    let replay_unbind = service
        .unbind(&unbind_request("binding-a", "unbind-a"))
        .expect("rejeu");
    assert_eq!(first_unbind, replay_unbind);
    assert!(!service.is_bound(&"binding-a".parse().expect("binding")));
}

#[test]
fn invoke_replay_returns_same_response() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");

    let first = service
        .invoke(&invoke_request(
            "binding-a",
            "put-1",
            "kv.put",
            "k",
            Some("v"),
        ))
        .expect("kv.put");
    let replay = service
        .invoke(&invoke_request(
            "binding-a",
            "put-1",
            "kv.put",
            "k",
            Some("v"),
        ))
        .expect("rejeu");
    assert_eq!(first, replay);
}

// ---------- Isolation ----------

#[test]
fn two_bindings_never_share_kv_namespace() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind a");
    service
        .bind(&bind_request("binding-b", "bind-b"))
        .expect("bind b");

    service
        .invoke(&invoke_request(
            "binding-a",
            "put-a",
            "kv.put",
            "shared",
            Some("v-a"),
        ))
        .expect("put a");

    let read_b = service
        .invoke(&invoke_request(
            "binding-b",
            "get-b",
            "kv.get",
            "shared",
            None,
        ))
        .expect("get b");
    assert_eq!(read_b.result, serde_json::json!({"found": false}));

    let read_a = service
        .invoke(&invoke_request(
            "binding-a",
            "get-a",
            "kv.get",
            "shared",
            None,
        ))
        .expect("get a");
    assert_eq!(
        read_a.result,
        serde_json::json!({"found": true, "value": "v-a"})
    );

    service
        .unbind(&unbind_request("binding-a", "unbind-a"))
        .expect("unbind a");
    service
        .bind(&bind_request("binding-a", "bind-a-2"))
        .expect("re-bind a");
    let after_purge = service
        .invoke(&invoke_request(
            "binding-a",
            "get-a-2",
            "kv.get",
            "shared",
            None,
        ))
        .expect("get après purge");
    assert_eq!(after_purge.result, serde_json::json!({"found": false}));
}

// ---------- Contrat de capacités ----------

#[test]
fn reference_declares_no_network_capability_and_fixture_is_valid() {
    let raw = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/apparatus.toml"))
        .expect("fixture apparatus.toml");
    let validated = validate_manifest_toml(&raw).expect("fixture valide");
    assert_eq!(
        validated.manifest.apparatus.id.as_str(),
        REFERENCE_APPARATUS_ID
    );

    let caps =
        apparatus_contracts::Capability::parse_list(&validated.manifest.capabilities.requires)
            .expect("capacités connues");
    assert!(!apparatus_contracts::requires_network(&caps));
    for name in &validated.manifest.capabilities.requires {
        assert!(
            !name.contains("network") && !name.contains("egress") && !name.contains("http"),
            "aucune capacité réseau : {name}"
        );
    }

    let schema = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/ui/settings.schema.json"
    ))
    .expect("fixture schéma UI");
    let parsed: serde_json::Value = serde_json::from_str(&schema).expect("schéma JSON valide");
    assert_eq!(parsed["type"], serde_json::json!("object"));
}

#[test]
fn unbind_unknown_binding_rejected_but_replay_survives() {
    let service = backend();
    // Binding jamais attaché : rejet en InvalidOperation.
    let err = service
        .unbind(&unbind_request("ghost", "unbind-ghost"))
        .expect_err("unbind inconnu rejeté");
    assert_eq!(
        format!("{err}"),
        "contract error: invalid operation: unknown binding"
    );

    // Cycle bind puis unbind : rejeu du même operation_id après purge.
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");
    let first = service
        .unbind(&unbind_request("binding-a", "unbind-a"))
        .expect("unbind");
    let replay = service
        .unbind(&unbind_request("binding-a", "unbind-a"))
        .expect("rejeu");
    assert_eq!(first, replay);

    // Nouvel operation_id après purge : rejet.
    service
        .unbind(&unbind_request("binding-a", "unbind-a-2"))
        .expect_err("second unbind rejeté");
}

#[test]
fn deterministic_operation_id_is_stable_without_rng() {
    // Même libellé ⇒ même operation_id, sans aléas.
    let first = deterministic_operation_id("stable-label");
    let second = deterministic_operation_id("stable-label");
    assert_eq!(first, second);

    // Libellés distincts ⇒ ids distincts.
    let other = deterministic_operation_id("other-label");
    assert_ne!(first, other);

    // UUID bien formé (version 4, variante RFC 4122).
    let text = first.to_string();
    assert_eq!(text.len(), 36);
    assert_eq!(&text[14..15], "4");
}

// ---------- Rejets invoke ----------

#[test]
fn invoke_rejects_unknown_operation_missing_key_and_unknown_binding() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");

    let err = service
        .invoke(&invoke_request(
            "binding-a",
            "unknown-op",
            "kv.unknown",
            "k",
            None,
        ))
        .expect_err("op inconnue rejetée");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "InvalidOperation attendu"
    );
    assert!(
        format!("{err}").contains("unknown reference operation"),
        "raison opération inconnue"
    );

    let manual = InvokeRequest {
        binding_id: "binding-a".parse().expect("binding id"),
        operation_id: deterministic_operation_id("missing-key"),
        operation: "kv.get".to_owned(),
        params: serde_json::json!({"value": "v"}),
    };
    let err = service.invoke(&manual).expect_err("clé manquante rejetée");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "InvalidOperation attendu"
    );

    let err = service
        .invoke(&invoke_request("ghost", "ghost-op", "kv.get", "k", None))
        .expect_err("binding inconnu rejeté");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "InvalidOperation attendu"
    );
    assert!(
        format!("{err}").contains("unknown binding"),
        "raison binding inconnu"
    );
}

#[test]
fn kv_delete_reports_removed_true_then_false() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");
    service
        .invoke(&invoke_request(
            "binding-a",
            "put-del",
            "kv.put",
            "k",
            Some("v"),
        ))
        .expect("put");
    let first = service
        .invoke(&invoke_request(
            "binding-a",
            "del-1",
            "kv.delete",
            "k",
            None,
        ))
        .expect("delete");
    assert_eq!(first.result, serde_json::json!({"removed": true}));
    let second = service
        .invoke(&invoke_request(
            "binding-a",
            "del-2",
            "kv.delete",
            "k",
            None,
        ))
        .expect("re-delete");
    assert_eq!(second.result, serde_json::json!({"removed": false}));
}

#[test]
fn kv_key_and_value_bounds_enforced() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");

    let err = service
        .invoke(&invoke_request(
            "binding-a",
            "empty-key",
            "kv.put",
            "",
            Some("v"),
        ))
        .expect_err("clé vide rejetée");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "clé vide en InvalidOperation"
    );

    let long_key = "k".repeat(MAX_KV_KEY_LEN + 1);
    let err = service
        .invoke(&invoke_request(
            "binding-a",
            "long-key",
            "kv.put",
            &long_key,
            Some("v"),
        ))
        .expect_err("clé trop longue rejetée");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "clé trop longue en InvalidOperation"
    );

    let big_value = "v".repeat(MAX_KV_VALUE_BYTES + 1);
    let err = service
        .invoke(&invoke_request(
            "binding-a",
            "big-value",
            "kv.put",
            "k",
            Some(big_value.as_str()),
        ))
        .expect_err("valeur trop grosse rejetée");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::PayloadTooLarge { .. })
        ),
        "valeur trop grosse en PayloadTooLarge"
    );
}

#[test]
fn configure_unknown_binding_rejected() {
    let service = backend();
    let err = service
        .configure(&configure_request("ghost", "cfg-ghost"))
        .expect_err("configure inconnu rejeté");
    assert!(
        matches!(
            err,
            ReferenceKvError::Contract(ApparatusError::InvalidOperation { .. })
        ),
        "InvalidOperation attendu"
    );
    assert!(
        format!("{err}").contains("unknown binding"),
        "raison binding inconnu"
    );
}
