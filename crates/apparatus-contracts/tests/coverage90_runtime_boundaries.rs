//! QA coverage90 C: pure runtime lexical/structural rules, not admission authority.
use apparatus_contracts::runtime::{
    parse_runtime_json, validate_runtime_dns_name, validate_runtime_oci_pin, validate_runtime_uid,
    RuntimeCounterV1, RuntimeErrorCodeV1, RuntimeInstanceCursorPayloadV1,
    RuntimeListCursorPayloadV1, RuntimeServiceRoleV1, RuntimeUnixMsV1, MAX_RUNTIME_BODY_BYTES,
};
use serde_json::{json, Value};
use uuid::Uuid;

#[test]
fn dns_label_and_subdomain_limits_are_distinct_and_never_normalized() {
    assert_eq!(validate_runtime_dns_name(&"a".repeat(63), false), Ok(()));
    assert_eq!(
        validate_runtime_dns_name(&"a".repeat(64), false),
        Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
    );
    let exact = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    ]
    .join(".");
    assert_eq!(exact.len(), 253);
    assert_eq!(validate_runtime_dns_name(&exact, true), Ok(()));
    assert_eq!(
        validate_runtime_dns_name(&format!("{exact}d"), true),
        Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
    );
    assert_eq!(
        validate_runtime_dns_name("service.cluster.local", true),
        Ok(())
    );
    assert_eq!(
        validate_runtime_dns_name("service.cluster.local", false),
        Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
    );
    for invalid in [
        "", "Service", "-service", "service-", "a..b", ".a", "a.", "é", "a_b", "a/b",
    ] {
        assert_eq!(
            validate_runtime_dns_name(invalid, true),
            Err(RuntimeErrorCodeV1::RuntimeInvalidRequest),
            "{invalid:?}"
        );
    }
}

#[test]
fn opaque_uid_limit_counts_bytes_and_rejects_control_characters() {
    assert_eq!(validate_runtime_uid(&"u".repeat(128)), Ok(()));
    assert_eq!(validate_runtime_uid(&"é".repeat(64)), Ok(()));
    for invalid in [
        String::new(),
        "u".repeat(129),
        "é".repeat(65),
        "pod\nuid".into(),
        "pod\0uid".into(),
        "pod\u{7f}uid".into(),
    ] {
        assert_eq!(
            validate_runtime_uid(&invalid),
            Err(RuntimeErrorCodeV1::RuntimeInvalidRequest),
            "{invalid:?}"
        );
    }
}

#[test]
fn oci_pin_accepts_only_canonical_repository_port_and_lowercase_digest() {
    let digest = format!("sha256:{}", "a0".repeat(32));
    for repository in [
        "registry.example.test/team/image",
        "localhost:5000/team/image",
        "registry.example.test:65535/image_v1.2",
    ] {
        assert_eq!(
            validate_runtime_oci_pin(&format!("{repository}@{digest}")),
            Ok(())
        );
    }
    for repository in [
        "https://registry.example.test/image",
        "registry.example.test/image:latest",
        "Registry.example.test/image",
        "registry.example.test/Image",
        "registry.example.test:0/image",
        "registry.example.test:05000/image",
        "registry.example.test:65536/image",
        "registry.example.test:/image",
        "registry.example.test/",
        "registry.example.test/a//b",
        "registry.example.test/../image",
        "registry.example.test/./image",
        "registry.example.test/image/",
        "registry.example.test/é",
        "image",
    ] {
        assert_eq!(
            validate_runtime_oci_pin(&format!("{repository}@{digest}")),
            Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted),
            "{repository}"
        );
    }
    for invalid in [
        "registry.example.test/image:latest".to_owned(),
        format!("registry.example.test/image@{}", digest.to_uppercase()),
        format!("registry.example.test/image@{digest}@{digest}"),
        format!("registry.example.test/{}@{digest}", "a".repeat(2048)),
    ] {
        assert_eq!(
            validate_runtime_oci_pin(&invalid),
            Err(RuntimeErrorCodeV1::RuntimeReleaseNotAdmitted)
        );
    }
}

#[test]
fn strict_json_rejects_nested_duplicate_names_even_inside_array_objects() {
    for bytes in [
        br#"{"outer":[{"x":1,"x":2}]}"#.as_slice(),
        br#"{"outer":{"x":1,"\u0078":2}}"#.as_slice(),
        b"{} {}".as_slice(),
        br#"{"unterminated": [1,}"#.as_slice(),
    ] {
        assert_eq!(
            parse_runtime_json::<Value>(bytes),
            Err(RuntimeErrorCodeV1::RuntimeInvalidRequest)
        );
    }
    assert_eq!(
        parse_runtime_json::<Value>(br#"{"a":[{"x":1},{"x":2}],"b":{"x":3}}"#),
        Ok(json!({"a":[{"x":1},{"x":2}],"b":{"x":3}}))
    );
}

#[test]
fn strict_json_body_budget_accepts_exact_utf8_size_and_refuses_one_extra_byte() {
    let mut bytes = vec![b' '; MAX_RUNTIME_BODY_BYTES - 2];
    bytes.extend_from_slice(b"{}");
    assert_eq!(parse_runtime_json::<Value>(&bytes), Ok(json!({})));
    bytes.push(b' ');
    assert_eq!(
        parse_runtime_json::<Value>(&bytes),
        Err(RuntimeErrorCodeV1::RuntimePayloadTooLarge)
    );
}

fn cursor() -> RuntimeListCursorPayloadV1 {
    RuntimeListCursorPayloadV1 {
        schema_version: 1,
        scan_id: Uuid::from_u128(1),
        project_id: Uuid::from_u128(2),
        principal: RuntimeServiceRoleV1::Projector.principal().into(),
        after_binding_id: Uuid::from_u128(3),
        upper_binding_id: Uuid::from_u128(4),
        page_size: 200,
        issued_at_unix_ms: RuntimeUnixMsV1::new(1000).expect("time"),
        expires_at_unix_ms: RuntimeUnixMsV1::new(61_000).expect("time"),
    }
}

#[test]
fn cursor_preimage_is_canonical_and_not_an_authority_token() {
    let payload = cursor();
    let expected = concat!(
        "{\"after_binding_id\":\"00000000-0000-0000-0000-000000000003\",",
        "\"expires_at_unix_ms\":\"61000\",\"issued_at_unix_ms\":\"1000\",\"page_size\":200,",
        "\"principal\":\"spiffe://cluster.local/ns/aiforall-apparatus/sa/controller\",",
        "\"project_id\":\"00000000-0000-0000-0000-000000000002\",",
        "\"scan_id\":\"00000000-0000-0000-0000-000000000001\",\"schema_version\":1,",
        "\"upper_binding_id\":\"00000000-0000-0000-0000-000000000004\"}"
    );
    assert_eq!(
        payload.canonical_bytes().expect("structural cursor"),
        expected.as_bytes()
    );
    let mut changed = payload;
    changed.principal.push('/');
    assert_eq!(
        changed.canonical_bytes(),
        Err(RuntimeErrorCodeV1::RuntimeCursorInvalid)
    );
}

#[test]
fn cursor_requires_bounded_page_exact_lifetime_ordered_range_and_supported_version() {
    for page_size in [0, 201, u16::MAX] {
        let mut invalid = cursor();
        invalid.page_size = page_size;
        assert_eq!(
            invalid.canonical_bytes(),
            Err(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
    }
    for expiry in [999, 60_999, 61_001] {
        let mut invalid = cursor();
        invalid.expires_at_unix_ms = RuntimeUnixMsV1::new(expiry).expect("nonnegative time");
        assert_eq!(
            invalid.canonical_bytes(),
            Err(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
    }
    let mut invalid = cursor();
    invalid.after_binding_id = Uuid::from_u128(5);
    assert_eq!(
        invalid.canonical_bytes(),
        Err(RuntimeErrorCodeV1::RuntimeCursorInvalid)
    );
    invalid = cursor();
    invalid.schema_version = 2;
    assert_eq!(
        invalid.canonical_bytes(),
        Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion)
    );
}

#[test]
fn counter_json_rejects_non_string_types_instead_of_coercing_numeric_values() {
    for invalid in [json!(true), json!(null), json!(["1"]), json!({"value":"1"})] {
        assert!(
            serde_json::from_value::<RuntimeCounterV1>(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn instance_cursor_binds_the_binding_and_cannot_be_decoded_as_a_binding_cursor() {
    let cursor = RuntimeInstanceCursorPayloadV1 {
        schema_version: 1,
        scan_id: Uuid::from_u128(1),
        project_id: Uuid::from_u128(2),
        binding_id: Uuid::from_u128(3),
        principal: RuntimeServiceRoleV1::Lazaret.principal().into(),
        after_instance_id: Uuid::from_u128(4),
        upper_instance_id: Uuid::from_u128(5),
        page_size: 1,
        issued_at_unix_ms: RuntimeUnixMsV1::new(0).expect("time"),
        expires_at_unix_ms: RuntimeUnixMsV1::new(60_000).expect("time"),
    };
    let bytes = cursor.canonical_bytes().expect("instance cursor");
    let wire: Value = serde_json::from_slice(&bytes).expect("cursor JSON");
    assert_eq!(
        wire["binding_id"],
        json!("00000000-0000-0000-0000-000000000003")
    );
    assert!(parse_runtime_json::<RuntimeListCursorPayloadV1>(&bytes).is_err());
    let mut other_binding = cursor.clone();
    other_binding.binding_id = Uuid::from_u128(6);
    assert_ne!(
        other_binding.canonical_bytes().expect("different binding"),
        bytes
    );
    for invalid in 0..6 {
        let mut changed = cursor.clone();
        match invalid {
            0 => changed.schema_version = 2,
            1 => changed.page_size = 0,
            2 => changed.page_size = 201,
            3 => changed.expires_at_unix_ms = RuntimeUnixMsV1::new(59_999).expect("time"),
            4 => changed.after_instance_id = Uuid::from_u128(6),
            _ => changed.principal = "spiffe://cluster.local/ns/default/sa/default".into(),
        }
        let expected = if invalid == 0 {
            RuntimeErrorCodeV1::RuntimeUnsupportedVersion
        } else {
            RuntimeErrorCodeV1::RuntimeCursorInvalid
        };
        assert_eq!(
            changed.canonical_bytes(),
            Err(expected),
            "invalid cursor case {invalid}"
        );
    }
}
