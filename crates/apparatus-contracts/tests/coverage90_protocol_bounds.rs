//! QA coverage90 C: limits apply to compact UTF-8 JSON, not raw string length.
use std::collections::BTreeMap;

use apparatus_contracts::{
    protocol::validate_short_text, ApparatusError, BindRequest, ConfigureRequest, InvokeRequest,
    InvokeResponse, OperationId, MAX_CONFIG_BYTES, MAX_ID_LEN, MAX_OPERATION_NAME_LEN,
    MAX_PAYLOAD_BYTES,
};
use serde_json::{json, Value};

fn invoke(operation: &str, params: Value) -> InvokeRequest {
    InvokeRequest {
        binding_id: "binding-42".parse().expect("binding"),
        operation_id: OperationId::from_bytes([42; 16]),
        operation: operation.into(),
        params,
    }
}

#[test]
fn invoke_name_counts_unicode_characters_while_payload_counts_serialized_bytes() {
    let mut request = invoke(&"é".repeat(MAX_OPERATION_NAME_LEN), Value::Null);
    assert_eq!(request.validate(), Ok(()));
    request.operation.push('é');
    assert!(matches!(
        request.validate(),
        Err(ApparatusError::InvalidOperation { .. })
    ));
    request.operation.clear();
    assert!(matches!(
        request.validate(),
        Err(ApparatusError::InvalidOperation { .. })
    ));
}

#[test]
fn request_and_response_accept_exact_payload_bound_and_report_one_byte_over() {
    // JSON string quotes consume the two remaining bytes.
    let mut request = invoke("kv.set", json!("x".repeat(MAX_PAYLOAD_BYTES - 2)));
    assert_eq!(request.validate(), Ok(()));
    let mut response = InvokeResponse {
        binding_id: request.binding_id.clone(),
        operation_id: request.operation_id,
        result: request.params.clone(),
    };
    assert_eq!(response.validate(), Ok(()));
    request.params = json!("x".repeat(MAX_PAYLOAD_BYTES - 1));
    response.result = request.params.clone();
    let expected = ApparatusError::PayloadTooLarge {
        max: MAX_PAYLOAD_BYTES,
        actual: MAX_PAYLOAD_BYTES + 1,
    };
    assert_eq!(request.validate(), Err(expected.clone()));
    assert_eq!(response.validate(), Err(expected));
}

#[test]
fn escaped_payload_size_uses_wire_bytes_not_unescaped_character_count() {
    // Each newline is encoded as two bytes, plus two string quotes.
    let request = invoke("kv.set", json!("\n".repeat(MAX_PAYLOAD_BYTES / 2)));
    assert_eq!(
        request.validate(),
        Err(ApparatusError::PayloadTooLarge {
            max: MAX_PAYLOAD_BYTES,
            actual: MAX_PAYLOAD_BYTES + 2
        })
    );
}

#[test]
fn bind_and_configure_share_exact_configuration_budget_including_object_framing() {
    // {"x":"..."} contributes eight bytes besides the value content.
    let config = BTreeMap::from([("x".into(), json!("c".repeat(MAX_CONFIG_BYTES - 8)))]);
    let mut bind = BindRequest {
        binding_id: "binding-42".parse().expect("binding"),
        operation_id: OperationId::from_bytes([42; 16]),
        apparatus_id: "io.aiforall.plugin".parse().expect("apparatus id"),
        release_digest: apparatus_contracts::digest_str("coverage90-release"),
        config: Some(config.clone()),
    };
    let mut configure = ConfigureRequest {
        binding_id: bind.binding_id.clone(),
        operation_id: bind.operation_id,
        config,
    };
    assert_eq!(bind.validate(), Ok(()));
    assert_eq!(configure.validate(), Ok(()));
    configure
        .config
        .insert("x".into(), json!("c".repeat(MAX_CONFIG_BYTES - 7)));
    bind.config = Some(configure.config.clone());
    let expected = ApparatusError::ConfigTooLarge {
        max: MAX_CONFIG_BYTES,
        actual: MAX_CONFIG_BYTES + 1,
    };
    assert_eq!(bind.validate(), Err(expected.clone()));
    assert_eq!(configure.validate(), Err(expected));
}

#[test]
fn invoke_wire_defaults_missing_params_to_null_but_rejects_unknown_authority_fields() {
    let value = json!({"binding_id":"binding-42", "operation_id":"00000000-0000-4000-8000-000000000042", "operation":"kv.get"});
    let parsed: InvokeRequest = serde_json::from_value(value.clone()).expect("optional params");
    assert_eq!(parsed.params, Value::Null);
    assert_eq!(parsed.validate(), Ok(()));
    let mut extra = value;
    extra["trusted"] = json!(true);
    assert!(serde_json::from_value::<InvokeRequest>(extra).is_err());
}

#[test]
fn short_text_bound_is_in_characters_without_ascii_only_id_restriction() {
    assert_eq!(validate_short_text(&"界".repeat(MAX_ID_LEN)), Ok(()));
    assert_eq!(
        validate_short_text(&"界".repeat(MAX_ID_LEN + 1)),
        Err(ApparatusError::IdTooLong {
            max: MAX_ID_LEN,
            actual: MAX_ID_LEN + 1
        })
    );
}
