//! Apparatus P2 — T6 ports P0 (unit, sans Docker).
//!
//! Double in-process déterministe : cycle bind/configure/unbind/teardown,
//! rejeu du même `operation_id` sans second effet (`applied = false`).

use std::collections::BTreeMap;

use apparatus_contracts::{
    ApparatusId, ApparatusRuntime, BindRequest, BindingId, ConfigureRequest, ReleaseDigest,
    UnbindRequest,
};
use manifesto_infra::apparatus_runtime::{derived_operation_id, InProcessApparatusRuntime};
use uuid::Uuid;

fn sample_digest() -> ReleaseDigest {
    ReleaseDigest::from_bytes(&[0x42; 32])
}

fn sample_binding() -> (Uuid, BindingId, ApparatusId) {
    let component_id = Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
    let binding = BindingId::new(&component_id.to_string()).expect("uuid binding");
    let apparatus = ApparatusId::new("io.aiforall.taskboard").expect("apparatus id");
    (component_id, binding, apparatus)
}

#[test]
fn t6_bind_configure_unbind_teardown_cycle_and_observe() {
    let runtime = InProcessApparatusRuntime::new();
    let (component_id, binding, apparatus) = sample_binding();
    let digest = sample_digest();
    let bind_op = derived_operation_id(component_id, 1, "bind");
    let configure_op = derived_operation_id(component_id, 1, "configure");
    let unbind_op = derived_operation_id(component_id, 1, "unbind");

    let bind = runtime
        .bind(&BindRequest {
            binding_id: binding.clone(),
            apparatus_id: apparatus,
            release_digest: digest.clone(),
            operation_id: bind_op,
            config: None,
        })
        .expect("bind");
    assert!(bind.applied, "T6 RED : premier bind doit appliquer");
    assert_eq!(runtime.instance_count(), 1);

    let observed = runtime.observe(&binding).expect("observe après bind");
    assert_eq!(observed.digest.as_deref(), Some(digest.as_str()));
    assert_eq!(observed.generation, 1);

    let configure = runtime
        .configure(&ConfigureRequest {
            binding_id: binding.clone(),
            operation_id: configure_op,
            config: BTreeMap::new(),
        })
        .expect("configure");
    assert!(configure.applied);

    let unbind = runtime
        .unbind(&UnbindRequest {
            binding_id: binding.clone(),
            operation_id: unbind_op,
        })
        .expect("unbind");
    assert!(unbind.applied);
    assert_eq!(runtime.instance_count(), 0);

    runtime.teardown(&binding).expect("teardown");
    runtime.teardown(&binding).expect("teardown idempotent");
}

#[test]
fn t6_retry_same_operation_id_does_not_double_apply() {
    let runtime = InProcessApparatusRuntime::new();
    let (component_id, binding, apparatus) = sample_binding();
    let digest = sample_digest();
    let bind_op = derived_operation_id(component_id, 1, "bind");
    let configure_op = derived_operation_id(component_id, 1, "configure");
    let unbind_op = derived_operation_id(component_id, 1, "unbind");

    let first = runtime
        .bind(&BindRequest {
            binding_id: binding.clone(),
            apparatus_id: apparatus.clone(),
            release_digest: digest.clone(),
            operation_id: bind_op,
            config: None,
        })
        .expect("bind");
    assert!(first.applied);
    assert_eq!(runtime.applied_bind_count(), 1);

    let replay = runtime
        .bind(&BindRequest {
            binding_id: binding.clone(),
            apparatus_id: apparatus,
            release_digest: digest,
            operation_id: bind_op,
            config: None,
        })
        .expect("bind replay");
    assert!(
        !replay.applied,
        "T6 RED : rejeu du même operation_id => applied=false"
    );
    assert_eq!(runtime.applied_bind_count(), 1);
    assert_eq!(runtime.instance_count(), 1);

    runtime
        .configure(&ConfigureRequest {
            binding_id: binding.clone(),
            operation_id: configure_op,
            config: BTreeMap::new(),
        })
        .expect("configure");
    let configure_replay = runtime
        .configure(&ConfigureRequest {
            binding_id: binding.clone(),
            operation_id: configure_op,
            config: BTreeMap::new(),
        })
        .expect("configure replay");
    assert!(!configure_replay.applied);

    runtime
        .unbind(&UnbindRequest {
            binding_id: binding.clone(),
            operation_id: unbind_op,
        })
        .expect("unbind");
    let unbind_replay = runtime
        .unbind(&UnbindRequest {
            binding_id: binding,
            operation_id: unbind_op,
        })
        .expect("unbind replay");
    assert!(!unbind_replay.applied);
}

#[test]
fn t6_derived_operation_id_is_stable_and_verb_distinct() {
    let component_id = Uuid::from_u128(7);
    let a = derived_operation_id(component_id, 1, "bind");
    let b = derived_operation_id(component_id, 1, "bind");
    let c = derived_operation_id(component_id, 1, "teardown");
    let d = derived_operation_id(component_id, 2, "bind");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}
