//! Micro-tests P0.1 — backend KV de référence (in-memory, sans Docker).
//!
//! Justification assumée (non-bug, sans refactor) : `configure` persiste dans
//! la map des bindings mais n'expose aucun getter — le cycle de vie
//! (rejet après `unbind`) en est la seule preuve observable en P0.

use apparatus_contracts::{BindRequest, ConfigureRequest};
use apparatus_reference_kv::{
    deterministic_operation_id, InMemoryKvStore, ReferenceKvBackend, REFERENCE_APPARATUS_ID,
};

fn backend() -> ReferenceKvBackend<InMemoryKvStore> {
    ReferenceKvBackend::new(InMemoryKvStore::new())
}

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

#[test]
fn configure_rejected_after_unbind_proves_lifecycle_binding() {
    let service = backend();
    service
        .bind(&bind_request("binding-a", "bind-a"))
        .expect("bind");
    service
        .unbind(&apparatus_contracts::UnbindRequest {
            binding_id: "binding-a".parse().expect("binding id"),
            operation_id: deterministic_operation_id("unbind-a"),
        })
        .expect("unbind");
    let req = ConfigureRequest {
        binding_id: "binding-a".parse().expect("binding id"),
        operation_id: deterministic_operation_id("cfg-after-unbind"),
        config: std::collections::BTreeMap::new(),
    };
    service
        .configure(&req)
        .expect_err("configure après unbind rejeté");
}

#[test]
fn bind_rejects_oversized_initial_config() {
    let service = backend();
    let mut config = std::collections::BTreeMap::new();
    config.insert(
        "blob".to_owned(),
        serde_json::Value::String("v".repeat(apparatus_contracts::MAX_CONFIG_BYTES + 1)),
    );
    let req = BindRequest {
        config: Some(config),
        ..bind_request("binding-a", "bind-big")
    };
    let err = service.bind(&req).expect_err("config initiale trop grosse");
    assert!(
        matches!(
            err,
            apparatus_reference_kv::ReferenceKvError::Contract(
                apparatus_contracts::ApparatusError::ConfigTooLarge { .. }
            )
        ),
        "ConfigTooLarge attendu, obtenu : {err}"
    );
}
