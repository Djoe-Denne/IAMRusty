//! QA coverage90 C: validated identities at the wire and closed capability grants.
use apparatus_contracts::{
    requires_network, ApparatusError, ApparatusId, BindingId, Capability, OperationId,
    ReleaseDigest, MAX_ID_LEN,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

fn assert_string_wire<T: DeserializeOwned + Serialize>(text: &str) {
    let parsed: T = serde_json::from_value(json!(text)).expect("valid identity wire");
    assert_eq!(
        serde_json::to_value(parsed).expect("identity JSON"),
        json!(text)
    );
}

#[test]
fn binding_ids_enforce_ascii_alphabet_and_exact_length_at_both_boundaries() {
    for valid in ["Binding_42.v1-az".to_owned(), "b".repeat(MAX_ID_LEN)] {
        let parsed = BindingId::new(&valid).expect("opaque ASCII binding");
        assert_eq!(parsed.to_string(), valid);
        assert_string_wire::<BindingId>(&valid);
    }
    for invalid in ["", "a/b", "a b", "a:b", "a\nb", "é", "绑定"] {
        assert!(
            matches!(
                BindingId::new(invalid),
                Err(ApparatusError::InvalidId { .. })
            ),
            "{invalid:?}"
        );
        assert!(
            serde_json::from_value::<BindingId>(json!(invalid)).is_err(),
            "wire bypass: {invalid:?}"
        );
    }
    assert_eq!(
        BindingId::new(&"b".repeat(MAX_ID_LEN + 1)),
        Err(ApparatusError::IdTooLong {
            max: MAX_ID_LEN,
            actual: MAX_ID_LEN + 1
        })
    );
}

#[test]
fn reverse_domain_id_exact_bound_and_wire_validation_do_not_normalize_input() {
    let exact = format!("io.{}", "a".repeat(MAX_ID_LEN - 3));
    assert_string_wire::<ApparatusId>(&exact);
    for invalid in [
        "IO.aiforall.plugin",
        "io.plugin/child",
        "io.plugin ",
        "io.é",
        ".io",
        "io.",
    ] {
        assert!(
            serde_json::from_value::<ApparatusId>(json!(invalid)).is_err(),
            "{invalid:?}"
        );
    }
    assert!(serde_json::from_value::<ApparatusId>(json!(format!("{exact}a"))).is_err());
    for non_string in [Value::Null, json!(42), json!(["io.plugin"])] {
        assert!(serde_json::from_value::<ApparatusId>(non_string.clone()).is_err());
        assert!(serde_json::from_value::<BindingId>(non_string).is_err());
    }
}

#[test]
fn operation_id_byte_vectors_force_uuid_v4_and_rfc_variant_without_changing_other_bits() {
    for (bytes, expected) in [
        ([0; 16], "00000000-0000-4000-8000-000000000000"),
        ([255; 16], "ffffffff-ffff-4fff-bfff-ffffffffffff"),
    ] {
        let id = OperationId::from_bytes(bytes);
        assert_eq!(id.to_string(), expected);
        assert_string_wire::<OperationId>(expected);
        assert_eq!(expected.parse::<OperationId>().expect("UUID"), id);
    }
    for invalid in ["", "operation-42", "00000000-0000-4000-8000-00000000000g"] {
        assert!(matches!(
            invalid.parse::<OperationId>(),
            Err(ApparatusError::InvalidId { .. })
        ));
        assert!(serde_json::from_value::<OperationId>(json!(invalid)).is_err());
    }
}

#[test]
fn digest_byte_encoding_is_lowercase_fixed_width_and_wire_cannot_skip_validation() {
    let bytes = [0xab; 32];
    let expected = format!("sha256:{}", "ab".repeat(32));
    let digest = ReleaseDigest::from_bytes(&bytes);
    assert_eq!(digest.to_string(), expected);
    assert_string_wire::<ReleaseDigest>(&expected);
    assert_string_wire::<ReleaseDigest>(&format!("sha256:{}", "00".repeat(32)));
    for invalid in [
        format!("sha256:{}", "ab".repeat(31)),
        format!("sha256:{}", "ab".repeat(33)),
        format!("sha256:{}", "AB".repeat(32)),
        format!("sha256:{}", "gg".repeat(32)),
        format!("SHA256:{}", "ab".repeat(32)),
    ] {
        assert!(
            serde_json::from_value::<ReleaseDigest>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn capability_names_and_network_grants_are_explicit_not_inferred_from_runtime_invoke() {
    let cases = [
        ("project.read", Capability::ProjectRead, false),
        ("storage.kv.read", Capability::StorageKvRead, false),
        ("storage.kv.write", Capability::StorageKvWrite, false),
        ("connector.fetch", Capability::ConnectorFetch, true),
        ("runtime.invoke", Capability::RuntimeInvoke, false),
    ];
    for (wire, capability, network) in cases {
        assert_eq!(wire.parse::<Capability>(), Ok(capability));
        assert_string_wire::<Capability>(wire);
        assert_eq!(capability.to_string(), wire);
        assert_eq!(capability.is_network(), network);
    }
    assert_eq!(Capability::all().len(), cases.len());
    assert!(!requires_network(&[]));
    assert!(!requires_network(&[
        Capability::RuntimeInvoke,
        Capability::StorageKvWrite
    ]));
    assert!(requires_network(&[
        Capability::ProjectRead,
        Capability::ConnectorFetch
    ]));
    assert!(requires_network(&[
        Capability::ConnectorFetch,
        Capability::RuntimeInvoke
    ]));
}

#[test]
fn unknown_capability_in_a_list_fails_closed_instead_of_returning_a_partial_grant() {
    for unknown in [
        "network.egress",
        "Project.read",
        "project.read ",
        "",
        "runtime.*",
    ] {
        let names = vec![
            "project.read".into(),
            unknown.into(),
            "storage.kv.write".into(),
        ];
        assert_eq!(
            Capability::parse_list(&names),
            Err(ApparatusError::UnknownCapability {
                name: unknown.into()
            })
        );
        assert!(serde_json::from_value::<Capability>(json!(unknown)).is_err());
    }
    let unknown = "é".repeat(101);
    assert_eq!(
        unknown.parse::<Capability>(),
        Err(ApparatusError::UnknownCapability {
            name: format!("{}…", "é".repeat(100))
        })
    );
}
