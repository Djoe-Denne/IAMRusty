//! Micro-tests P0.1 — contrats Apparatus (in-memory, déterministes, sans Docker).
//!
//! Ferme la réserve d'audit « `ReadyResponse` peu exercée ».
//! Justifications assumées (non-bugs, sans refactor) :
//! - Doublon KV : `InMemoryKv` (harness TEST-ONLY, feature `test-harness`) vs
//!   `InMemoryKvStore` (backend de référence injecté) — deux rôles distincts.
//! - Identités au bind : le harness P0 n'admet pas (pas de refus sur
//!   `apparatus_id`/`release_digest`) — les contrats valident le wire, pas l'admission.

use apparatus_contracts::{
    digest_str, ConfigureRequest, DiscoveryEndpoints, ReadyResponse, BIND_PATH, CONFIGURE_PATH,
    HEALTH_PATH, INVOKE_PATH, READY_PATH, UNBIND_PATH,
};

#[test]
fn ready_response_serde_roundtrip_and_deny_unknown() {
    let ready = ReadyResponse {
        ready: true,
        apparatus_id: "io.aiforall.reference-kv".parse().expect("apparatus id"),
        release_digest: digest_str("release").as_str().parse().expect("digest"),
    };
    let json = serde_json::to_value(&ready).expect("sérialisation ready");
    assert_eq!(json["ready"], serde_json::json!(true));
    let back: ReadyResponse = serde_json::from_value(json).expect("roundtrip ready");
    assert_eq!(ready, back);

    let bad = serde_json::json!({
        "ready": true,
        "apparatus_id": "io.aiforall.reference-kv",
        "release_digest": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "host": "x"
    });
    assert!(
        serde_json::from_value::<ReadyResponse>(bad).is_err(),
        "ready champ inconnu rejeté"
    );
}

#[test]
fn discovery_endpoints_default_paths_are_canonical() {
    let endpoints = DiscoveryEndpoints::default();
    assert_eq!(endpoints.health, HEALTH_PATH);
    assert_eq!(endpoints.ready, READY_PATH);
    assert_eq!(endpoints.bind, BIND_PATH);
    assert_eq!(endpoints.configure, CONFIGURE_PATH);
    assert_eq!(endpoints.invoke, INVOKE_PATH);
    assert_eq!(endpoints.unbind, UNBIND_PATH);
}

#[test]
fn configure_empty_config_is_valid_and_bounded() {
    let req = ConfigureRequest {
        binding_id: "binding-a".parse().expect("binding id"),
        operation_id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"
            .parse()
            .expect("operation id"),
        config: std::collections::BTreeMap::new(),
    };
    req.validate().expect("config vide valide");
}
