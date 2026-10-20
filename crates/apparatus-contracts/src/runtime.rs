//! Versioned, cluster-independent runtime control contract (ADR 0012 §13.1).
//!
//! Transport adapters must authenticate the service principal before calling the
//! port. A DTO, cursor or hash is never proof of authority.

#![allow(missing_docs)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

pub const RUNTIME_SCHEMA_VERSION: u16 = 1;
pub const OPERATION_HASH_DOMAIN: &str = "urn:aiforall:apparatus:runtime:operation-hash:v1";
pub const EFFECT_HASH_DOMAIN: &str = "urn:aiforall:apparatus:runtime:effect-hash:v1";
pub const INVOKE_ISSUER: &str = "urn:aiforall:lazaret:apparatus-runtime:v1";
pub const INVOKE_AUDIENCE: &str = "urn:aiforall:apparatus:runtime:invoke:v1";
pub const INVOKE_PURPOSE: &str = "apparatus.runtime.invoke.v1";
pub const INVOKE_TYP: &str = "apparatus-runtime-invoke+jwt";
pub const WRITER_LEASE_TTL_MS: i64 = 10_000;
pub const WRITER_LEASE_RENEW_MS: i64 = 3_000;
pub const OBSERVATION_TTL_MS: i64 = 5_000;
pub const MAX_RUNTIME_BODY_BYTES: usize = 262_144;

/// Validate an opaque Kubernetes UID without assuming it is a UUID.
///
/// # Errors
/// Rejects empty, oversized or control-bearing values.
pub fn validate_runtime_uid(value: &str) -> Result<(), RuntimeErrorCodeV1> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
    }
    Ok(())
}

/// Validate a Kubernetes DNS label/subdomain without normalizing caller data.
///
/// # Errors
/// Rejects non-ASCII/lowercase names, empty labels and oversized DNS components.
pub fn validate_runtime_dns_name(value: &str, subdomain: bool) -> Result<(), RuntimeErrorCodeV1> {
    let max = if subdomain { 253 } else { 63 };
    if value.is_empty() || value.len() > max || (!subdomain && value.contains('.')) {
        return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
    }
    for label in value.split('.') {
        let bytes = label.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 63
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            || !bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
    }
    Ok(())
}

/// Parse a runtime envelope before serde can discard duplicate object keys.
///
/// # Errors
///
/// Rejects oversized bodies, duplicate names at every depth, out-of-profile
/// integer literals, malformed JSON, unknown fields and unsupported versions.
pub fn parse_runtime_json<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
) -> Result<T, RuntimeErrorCodeV1> {
    if bytes.len() > MAX_RUNTIME_BODY_BYTES {
        return Err(RuntimeErrorCodeV1::RuntimePayloadTooLarge);
    }
    reject_unsafe_integer_literals(bytes)?;
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    let value = StrictRuntimeJson::deserialize(&mut parser)
        .map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?
        .0;
    parser
        .end()
        .map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?;
    // Keep the protocol-version error distinct without relaxing unknown-field
    // validation. Every public envelope has a required numeric schema_version.
    if value
        .get("schema_version")
        .and_then(Value::as_u64)
        .is_some_and(|v| v != 1)
    {
        return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
    }
    serde_json::from_value(value).map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)
}

// serde_json otherwise converts integer literals larger than u64 into f64,
// losing the lexical information before Visitor::visit_f64 sees the value.
fn reject_unsafe_integer_literals(bytes: &[u8]) -> Result<(), RuntimeErrorCodeV1> {
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            index += 1;
            continue;
        }
        if byte == b'-' || byte.is_ascii_digit() {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || b"-.eE+".contains(&bytes[index]))
            {
                index += 1;
            }
            let token = &bytes[start..index];
            if !token.iter().any(|b| b".eE".contains(b)) {
                let integer = std::str::from_utf8(token)
                    .ok()
                    .and_then(|value| value.parse::<i64>().ok());
                if !integer.is_some_and(|value| {
                    (-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&value)
                }) {
                    return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
                }
            }
            continue;
        }
        index += 1;
    }
    Ok(())
}

struct StrictRuntimeJson(Value);

impl<'de> Deserialize<'de> for StrictRuntimeJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictRuntimeVisitor).map(Self)
    }
}

struct StrictRuntimeVisitor;
impl<'de> serde::de::Visitor<'de> for StrictRuntimeVisitor {
    type Value = Value;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("runtime JSON without duplicate keys or unsafe integer literals")
    }
    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Value, E> {
        if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&value) {
            return Err(E::custom("unsafe integer literal"));
        }
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Value, E> {
        if value > 9_007_199_254_740_991 {
            return Err(E::custom("unsafe integer literal"));
        }
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite number"))
    }
    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element::<StrictRuntimeJson>()? {
            values.push(value.0);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom("duplicate property"));
            }
            values.insert(key, access.next_value::<StrictRuntimeJson>()?.0);
        }
        Ok(Value::Object(values))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeCounterV1(u64);

impl RuntimeCounterV1 {
    pub fn new(value: u64) -> Result<Self, RuntimeErrorCodeV1> {
        if value > i64::MAX as u64 {
            return Err(RuntimeErrorCodeV1::RuntimeCounterExhausted);
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u64 {
        self.0
    }
    pub fn checked_next(self) -> Result<Self, RuntimeErrorCodeV1> {
        Self::new(
            self.0
                .checked_add(1)
                .ok_or(RuntimeErrorCodeV1::RuntimeCounterExhausted)?,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeUnixMsV1(i64);

impl RuntimeUnixMsV1 {
    pub fn new(value: i64) -> Result<Self, RuntimeErrorCodeV1> {
        if value < 0 {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> i64 {
        self.0
    }
}

fn decimal<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    let text = String::deserialize(deserializer)?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(serde::de::Error::custom("invalid runtime decimal string"));
    }
    text.parse::<i64>().map_err(serde::de::Error::custom)
}

impl Serialize for RuntimeCounterV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for RuntimeCounterV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        decimal(deserializer).map(|value| Self(value as u64))
    }
}
impl Serialize for RuntimeUnixMsV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for RuntimeUnixMsV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        decimal(deserializer).map(Self)
    }
}

fn version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u16, D::Error> {
    let value = u16::deserialize(deserializer)?;
    if value != RUNTIME_SCHEMA_VERSION {
        return Err(serde::de::Error::custom("unsupported runtime version"));
    }
    Ok(value)
}

fn operation_domain<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value != OPERATION_HASH_DOMAIN {
        return Err(serde::de::Error::custom("invalid operation hash domain"));
    }
    Ok(value)
}

fn effect_domain<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value != EFFECT_HASH_DOMAIN {
        return Err(serde::de::Error::custom("invalid effect hash domain"));
    }
    Ok(value)
}

/// Required nullable values: unlike ordinary serde Option, missing is an error.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

fn uid_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    validate_runtime_uid(&value).map_err(|_| serde::de::Error::custom("invalid opaque UID"))?;
    Ok(value)
}
fn dns_label<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    validate_runtime_dns_name(&value, false)
        .map_err(|_| serde::de::Error::custom("invalid DNS label"))?;
    Ok(value)
}
fn dns_subdomain<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    validate_runtime_dns_name(&value, true)
        .map_err(|_| serde::de::Error::custom("invalid DNS subdomain"))?;
    Ok(value)
}
fn proof_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.is_empty() || value.len() > 16_384 {
        return Err(serde::de::Error::custom("invalid proof size"));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RuntimeDigestV1(String);

impl RuntimeDigestV1 {
    pub fn new(value: String) -> Result<Self, RuntimeErrorCodeV1> {
        let Some(hex) = value.strip_prefix("sha256:") else {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        };
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for RuntimeDigestV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?)
            .map_err(|_| serde::de::Error::custom("invalid runtime digest"))
    }
}

macro_rules! dto {
    ($name:ident { $($(#[$attr:meta])* $field:ident : $ty:ty),* $(,)? }) => {
        // No Debug: some envelopes carry raw enrollment proof or delegation ticket.
        #[derive(Clone, PartialEq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name { $( $(#[$attr])* pub $field: $ty, )* }
    };
}

dto!(RuntimeCasV1 {
    generation: RuntimeCounterV1,
    revocation_epoch: RuntimeCounterV1,
    projection_version: RuntimeCounterV1,
    grant_revision: RuntimeCounterV1
});
dto!(RuntimeLeaseProofV1 {
    lease_id: Uuid,
    holder_id: Uuid,
    writer_fence: RuntimeCounterV1
});
dto!(RuntimeWriterLeaseRequestV1 {
    #[serde(deserialize_with = "version")] schema_version: u16,
    holder_id: Uuid, expected: RuntimeCasV1,
    #[serde(deserialize_with = "required_option")] renew: Option<RuntimeLeaseProofV1>
});
dto!(RuntimeWriterLeaseV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    proof: RuntimeLeaseProofV1,
    cas: RuntimeCasV1,
    issued_at_unix_ms: RuntimeUnixMsV1,
    expires_at_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeWriteGuardV1 {
    expected: RuntimeCasV1,
    lease: RuntimeLeaseProofV1
});
dto!(RuntimeWriteAckV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    cas: RuntimeCasV1,
    writer_fence: RuntimeCounterV1
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeCursorV1(pub String);

dto!(RuntimeBindingPageV1 {
    #[serde(deserialize_with = "version")] schema_version: u16,
    items: Vec<RuntimeBindingSnapshotV1>,
    #[serde(deserialize_with = "required_option")] next_cursor: Option<RuntimeCursorV1>
});
dto!(RuntimeListCursorPayloadV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    scan_id: Uuid,
    project_id: Uuid,
    principal: String,
    after_binding_id: Uuid,
    upper_binding_id: Uuid,
    page_size: u16,
    issued_at_unix_ms: RuntimeUnixMsV1,
    expires_at_unix_ms: RuntimeUnixMsV1
});

impl RuntimeListCursorPayloadV1 {
    /// Exact RFC 8785 cursor preimage, with finite schema/page/TTL bounds.
    ///
    /// # Errors
    /// Rejects unsupported versions, invalid page sizes, malformed principal or
    /// cursor lifetime. This codec never grants authority or freshness.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RuntimeErrorCodeV1> {
        if self.schema_version != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
        }
        if !(1..=200).contains(&self.page_size)
            || self
                .expires_at_unix_ms
                .get()
                .checked_sub(self.issued_at_unix_ms.get())
                != Some(60_000)
            || self.after_binding_id > self.upper_binding_id
            || RuntimeServiceRoleV1::from_verified_principal(&self.principal).is_err()
        {
            return Err(RuntimeErrorCodeV1::RuntimeCursorInvalid);
        }
        serde_jcs::to_vec(self).map_err(|_| RuntimeErrorCodeV1::RuntimeCursorInvalid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeServiceRoleV1 {
    #[serde(rename = "apparatus.runtime.projector.v1")]
    Projector,
    #[serde(rename = "apparatus.runtime.lazaret.v1")]
    Lazaret,
}
impl RuntimeServiceRoleV1 {
    pub const fn principal(self) -> &'static str {
        match self {
            Self::Projector => "spiffe://cluster.local/ns/aiforall-apparatus/sa/controller",
            Self::Lazaret => "spiffe://cluster.local/ns/aiforall-gateway/sa/gateway",
        }
    }
    pub fn from_verified_principal(principal: &str) -> Result<Self, RuntimeErrorCodeV1> {
        [Self::Projector, Self::Lazaret]
            .into_iter()
            .find(|role| role.principal() == principal)
            .ok_or(RuntimeErrorCodeV1::RuntimeServiceForbidden)
    }
}

dto!(RuntimeSourceV1 {
    instance: Uuid,
    binding: Uuid,
    release: RuntimeDigestV1,
    generation: RuntimeCounterV1,
    grant_revision: RuntimeCounterV1
});
dto!(RuntimeTargetV1 {
    binding_id: Uuid,
    generation: RuntimeCounterV1,
    release: RuntimeDigestV1,
    grant_revision: RuntimeCounterV1,
    revocation_epoch: RuntimeCounterV1
});
dto!(RuntimeOperationHashInputV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    #[serde(deserialize_with = "operation_domain")]
    domain: String,
    authorization_id: Uuid,
    project_id: Uuid,
    source: RuntimeSourceV1,
    source_revocation_epoch: RuntimeCounterV1,
    target: RuntimeTargetV1,
    operation_id: Uuid,
    operation: String,
    payload: Value,
    deadline_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeCapabilityInvokeV1 {
    binding_id: Uuid,
    operation_id: Uuid,
    operation: String,
    params: Value
});
dto!(RuntimeEffectHashInputV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    #[serde(deserialize_with = "effect_domain")]
    domain: String,
    operation_sha256: RuntimeDigestV1,
    execution_id: Uuid,
    effect_id: Uuid,
    instance_id: Uuid,
    invoke: RuntimeCapabilityInvokeV1
});

fn canonical_hash<T: Serialize>(input: &T) -> Result<RuntimeDigestV1, RuntimeErrorCodeV1> {
    let value =
        serde_json::to_value(input).map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?;
    validate_hash_numbers(&value)?;
    let bytes = serde_jcs::to_vec(input).map_err(|_| RuntimeErrorCodeV1::RuntimeInvalidRequest)?;
    if bytes.len() > MAX_RUNTIME_BODY_BYTES {
        return Err(RuntimeErrorCodeV1::RuntimePayloadTooLarge);
    }
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(71);
    text.push_str("sha256:");
    use std::fmt::Write;
    for byte in digest {
        write!(&mut text, "{byte:02x}").map_err(|_| RuntimeErrorCodeV1::RuntimeInternal)?;
    }
    RuntimeDigestV1::new(text)
}

fn validate_hash_numbers(value: &Value) -> Result<(), RuntimeErrorCodeV1> {
    match value {
        Value::Number(number) => {
            if number
                .as_i64()
                .is_some_and(|n| !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&n))
                || number.as_u64().is_some_and(|n| n > 9_007_199_254_740_991)
            {
                return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
            }
        }
        Value::Array(values) => {
            for item in values {
                validate_hash_numbers(item)?;
            }
        }
        Value::Object(values) => {
            for item in values.values() {
                validate_hash_numbers(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl RuntimeOperationHashInputV1 {
    /// Hash the exact, domain-separated RFC 8785 preimage.
    ///
    /// # Errors
    /// Rejects unsupported domain/version, invalid JSON numbers and size excess.
    pub fn sha256(&self) -> Result<RuntimeDigestV1, RuntimeErrorCodeV1> {
        if self.schema_version != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
        }
        if self.domain != OPERATION_HASH_DOMAIN {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        canonical_hash(self)
    }
}
impl RuntimeEffectHashInputV1 {
    /// Hash the exact effect preimage, never reusing an operation hash domain.
    ///
    /// # Errors
    /// Rejects unsupported domain/version, invalid JSON numbers and size excess.
    pub fn sha256(&self) -> Result<RuntimeDigestV1, RuntimeErrorCodeV1> {
        if self.schema_version != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
        }
        if self.domain != EFFECT_HASH_DOMAIN {
            return Err(RuntimeErrorCodeV1::RuntimeInvalidRequest);
        }
        canonical_hash(self)
    }
}

dto!(RuntimeInvocationTicketV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    iss: String,
    aud: String,
    purpose: String,
    jti: Uuid,
    iat: i64,
    exp: i64,
    operation_sha256: RuntimeDigestV1,
    source: RuntimeSourceV1,
    source_revocation_epoch: RuntimeCounterV1,
    project_id: Uuid,
    target: RuntimeTargetV1,
    operation_id: Uuid,
    deadline_unix_ms: RuntimeUnixMsV1
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBindingStateV1 {
    Closed,
    Active,
    Draining,
    Tombstone,
}

dto!(RuntimeBindingSnapshotV1 {
    #[serde(deserialize_with = "version")] schema_version: u16,
    project_id: Uuid, binding_id: Uuid, cas: RuntimeCasV1, state: RuntimeBindingStateV1,
    release: RuntimeDigestV1, profile: String,
    #[serde(deserialize_with = "required_option")] observation: Option<RuntimeObservationSnapshotV1>,
    #[serde(deserialize_with = "required_option")] instances: Option<RuntimeInstancePageV1>
});
dto!(RuntimeObservationV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    guard: RuntimeWriteGuardV1,
    #[serde(deserialize_with = "uid_string")]
    service_uid: String,
    resource_generation: RuntimeCounterV1,
    observed_resource_generation: RuntimeCounterV1,
    #[serde(deserialize_with = "uid_string")]
    revision_uid: String,
    #[serde(deserialize_with = "dns_subdomain")]
    revision_name: String,
    release: RuntimeDigestV1,
    endpoint: String,
    route_ready: bool,
    observed_at_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeObservationSnapshotV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    cas: RuntimeCasV1,
    writer_fence: RuntimeCounterV1,
    service_uid: String,
    resource_generation: RuntimeCounterV1,
    observed_resource_generation: RuntimeCounterV1,
    revision_uid: String,
    revision_name: String,
    release: RuntimeDigestV1,
    endpoint: String,
    route_ready: bool,
    observed_at_unix_ms: RuntimeUnixMsV1,
    valid_until_unix_ms: RuntimeUnixMsV1
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeInstanceCursorV1(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeInstanceSelectorV1 {
    Page {
        #[serde(deserialize_with = "required_option")]
        cursor: Option<RuntimeInstanceCursorV1>,
        #[serde(deserialize_with = "required_option")]
        page_size: Option<u16>,
    },
    ByPodUid {
        pod_uid: String,
    },
    ByInstanceId {
        instance_id: Uuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeInstanceKindV1 {
    Pod,
    LegacyFence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeInstanceStateV1 {
    Current,
    Closed,
    Tombstone,
}
dto!(RuntimePodIdentityV1 {
    #[serde(deserialize_with = "uid_string")]
    pod_uid: String,
    #[serde(deserialize_with = "uid_string")]
    revision_uid: String,
    #[serde(deserialize_with = "dns_label")]
    namespace: String,
    #[serde(deserialize_with = "dns_subdomain")]
    service_account: String,
    #[serde(deserialize_with = "uid_string")]
    service_account_uid: String
});
dto!(RuntimeInstanceSnapshotV1 {
    #[serde(deserialize_with = "version")] schema_version: u16,
    project_id: Uuid, binding_id: Uuid, instance_id: Uuid,
    kind: RuntimeInstanceKindV1, state: RuntimeInstanceStateV1, cas: RuntimeCasV1, release: RuntimeDigestV1,
    #[serde(deserialize_with = "required_option")] pod: Option<RuntimePodIdentityV1>,
    #[serde(deserialize_with = "required_option")] writer_fence: Option<RuntimeCounterV1>,
    #[serde(deserialize_with = "required_option")] observed_at_unix_ms: Option<RuntimeUnixMsV1>,
    #[serde(deserialize_with = "required_option")] valid_until_unix_ms: Option<RuntimeUnixMsV1>,
    #[serde(deserialize_with = "required_option")] closed_at_unix_ms: Option<RuntimeUnixMsV1>
});
dto!(RuntimeInstancePageV1 {
    #[serde(deserialize_with = "version")] schema_version: u16,
    items: Vec<RuntimeInstanceSnapshotV1>,
    #[serde(deserialize_with = "required_option")] next_cursor: Option<RuntimeInstanceCursorV1>,
    read_at_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeInstanceCursorPayloadV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    scan_id: Uuid,
    project_id: Uuid,
    binding_id: Uuid,
    principal: String,
    after_instance_id: Uuid,
    upper_instance_id: Uuid,
    page_size: u16,
    issued_at_unix_ms: RuntimeUnixMsV1,
    expires_at_unix_ms: RuntimeUnixMsV1
});

impl RuntimeInstanceCursorPayloadV1 {
    /// Produce the exact instance-cursor JCS preimage, never a binding cursor.
    ///
    /// # Errors
    /// Rejects invalid version, principal, ordering, page size or absolute TTL.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RuntimeErrorCodeV1> {
        if self.schema_version != 1 {
            return Err(RuntimeErrorCodeV1::RuntimeUnsupportedVersion);
        }
        if !(1..=200).contains(&self.page_size)
            || self
                .expires_at_unix_ms
                .get()
                .checked_sub(self.issued_at_unix_ms.get())
                != Some(60_000)
            || self.after_instance_id > self.upper_instance_id
            || RuntimeServiceRoleV1::from_verified_principal(&self.principal).is_err()
        {
            return Err(RuntimeErrorCodeV1::RuntimeCursorInvalid);
        }
        serde_jcs::to_vec(self).map_err(|_| RuntimeErrorCodeV1::RuntimeCursorInvalid)
    }
}
dto!(RuntimeInstanceRegistrationV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    instance_id: Uuid,
    guard: RuntimeWriteGuardV1,
    state: RuntimeInstanceStateV1,
    #[serde(deserialize_with = "uid_string")]
    pod_uid: String,
    #[serde(deserialize_with = "uid_string")]
    revision_uid: String,
    #[serde(deserialize_with = "dns_label")]
    namespace: String,
    #[serde(deserialize_with = "dns_subdomain")]
    service_account: String,
    #[serde(deserialize_with = "uid_string")]
    service_account_uid: String,
    release: RuntimeDigestV1,
    observed_at_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeInvokeRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    target_binding_id: Uuid,
    expected_generation: RuntimeCounterV1,
    operation_id: Uuid,
    operation: String,
    payload: Value,
    deadline_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeEnrollmentRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    #[serde(deserialize_with = "uid_string")]
    pod_uid: String,
    #[serde(deserialize_with = "proof_string")]
    csr_pem: String,
    #[serde(deserialize_with = "proof_string")]
    projected_token: String
});
dto!(RuntimeEnrollmentResponseV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    instance_id: Uuid,
    pod_uid: String,
    certificate_chain_pem: String,
    expires_at_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeAcceptRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    instance_id: Uuid,
    #[serde(deserialize_with = "proof_string")]
    ticket: String,
    operation_id: Uuid,
    operation: String,
    payload: Value
});
dto!(RuntimeExecutionContextV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    execution_id: Uuid,
    instance_id: Uuid,
    authorization_id: Uuid,
    project_id: Uuid,
    source: RuntimeSourceV1,
    source_revocation_epoch: RuntimeCounterV1,
    target: RuntimeTargetV1,
    operation_sha256: RuntimeDigestV1,
    deadline_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeEffectRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    execution_id: Uuid,
    effect_id: Uuid,
    invoke: RuntimeCapabilityInvokeV1
});
dto!(RuntimeInstanceRevokeRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    expected: RuntimeCasV1,
    instance_id: Uuid
});
dto!(RuntimeBeginEffectRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    context: RuntimeExecutionContextV1,
    effect_id: Uuid,
    effect_sha256: RuntimeDigestV1,
    invoke: RuntimeCapabilityInvokeV1
});
dto!(RuntimeEffectReservationV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    project_id: Uuid,
    binding_id: Uuid,
    execution_id: Uuid,
    effect_id: Uuid,
    effect_sha256: RuntimeDigestV1,
    deadline_unix_ms: RuntimeUnixMsV1
});
dto!(RuntimeCompleteEffectRequestV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    reservation: RuntimeEffectReservationV1,
    completion_state: RuntimeEffectCompletionStateV1
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEffectCompletionStateV1 {
    Completed,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRetryClassV1 {
    Never,
    ReadOnly,
    ReconcileAfterRead,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEffectOutcomeV1 {
    NotStarted,
    MayHaveStarted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeErrorCodeV1 {
    RuntimeInvalidRequest,
    RuntimeUnsupportedVersion,
    RuntimeCursorInvalid,
    RuntimeCursorExpired,
    RuntimeUnauthenticated,
    RuntimeInvalidProof,
    RuntimeServiceForbidden,
    RuntimeCallerForbidden,
    RuntimeBindingClosed,
    RuntimeReleaseNotAdmitted,
    RuntimeIdentityRevoked,
    RuntimeHashMismatch,
    RuntimeLegacyDisabled,
    RuntimeBindingNotFound,
    RuntimeStaleState,
    RuntimeFenceConflict,
    RuntimeLeaseLost,
    RuntimeLeaseHeld,
    RuntimeReplay,
    RuntimeIdReused,
    RuntimePayloadTooLarge,
    RuntimeAuthorityUnavailable,
    RuntimeProjectionUnavailable,
    RuntimeRouteUnavailable,
    RuntimeCapacityExhausted,
    RuntimeKeyUnavailable,
    RuntimeDeadlineExceeded,
    RuntimeAuthorityProtocolError,
    RuntimeEffectOutcomeUnknown,
    RuntimeCounterExhausted,
    RuntimeInternal,
}
dto!(RuntimeControlErrorV1 {
    #[serde(deserialize_with = "version")]
    schema_version: u16,
    code: RuntimeErrorCodeV1,
    retry_class: RuntimeRetryClassV1,
    effect_outcome: RuntimeEffectOutcomeV1
});

impl std::fmt::Debug for RuntimeControlErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeControlErrorV1")
            .field("code", &self.code)
            .field("retry_class", &self.retry_class)
            .field("effect_outcome", &self.effect_outcome)
            .finish()
    }
}
impl std::fmt::Display for RuntimeControlErrorV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "runtime control error: {:?}", self.code)
    }
}
impl std::error::Error for RuntimeControlErrorV1 {}

impl RuntimeErrorCodeV1 {
    pub const fn http_status(self) -> u16 {
        use RuntimeErrorCodeV1::*;
        match self {
            RuntimeInvalidRequest
            | RuntimeUnsupportedVersion
            | RuntimeCursorInvalid
            | RuntimeCursorExpired => 400,
            RuntimeUnauthenticated | RuntimeInvalidProof => 401,
            RuntimeServiceForbidden
            | RuntimeCallerForbidden
            | RuntimeBindingClosed
            | RuntimeReleaseNotAdmitted
            | RuntimeIdentityRevoked
            | RuntimeHashMismatch
            | RuntimeLegacyDisabled => 403,
            RuntimeBindingNotFound => 404,
            RuntimeStaleState | RuntimeFenceConflict | RuntimeLeaseLost | RuntimeLeaseHeld
            | RuntimeReplay | RuntimeIdReused => 409,
            RuntimePayloadTooLarge => 413,
            RuntimeAuthorityUnavailable
            | RuntimeProjectionUnavailable
            | RuntimeRouteUnavailable
            | RuntimeCapacityExhausted
            | RuntimeKeyUnavailable => 503,
            RuntimeDeadlineExceeded => 504,
            RuntimeAuthorityProtocolError | RuntimeEffectOutcomeUnknown => 502,
            RuntimeCounterExhausted | RuntimeInternal => 500,
        }
    }
    pub fn error(
        self,
        role: RuntimeServiceRoleV1,
        outcome: RuntimeEffectOutcomeV1,
    ) -> RuntimeControlErrorV1 {
        use RuntimeErrorCodeV1::*;
        let retry_class = match self {
            RuntimeCursorExpired => RuntimeRetryClassV1::ReconcileAfterRead,
            RuntimeStaleState | RuntimeFenceConflict | RuntimeLeaseLost | RuntimeLeaseHeld
                if role == RuntimeServiceRoleV1::Projector =>
            {
                RuntimeRetryClassV1::ReconcileAfterRead
            }
            RuntimeAuthorityUnavailable
            | RuntimeProjectionUnavailable
            | RuntimeRouteUnavailable
            | RuntimeCapacityExhausted
            | RuntimeKeyUnavailable => RuntimeRetryClassV1::ReadOnly,
            _ => RuntimeRetryClassV1::Never,
        };
        RuntimeControlErrorV1 {
            schema_version: 1,
            code: self,
            retry_class,
            effect_outcome: outcome,
        }
    }
}

/// Trusted adapter context, intentionally not deserializable from a request.
#[derive(Debug, Clone)]
pub struct RuntimeControlCallerV1 {
    pub role: RuntimeServiceRoleV1,
}

pub type RuntimeControlFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, RuntimeControlErrorV1>> + Send + 'a>>;

/// Writer-authoritative port; adapters must not substitute cached allow decisions.
pub trait RuntimeControlPort: Send + Sync {
    fn list_bindings<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        cursor: Option<&'a RuntimeCursorV1>,
        page_size: Option<u16>,
    ) -> RuntimeControlFuture<'a, RuntimeBindingPageV1>;
    fn get_binding<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instances: &'a RuntimeInstanceSelectorV1,
    ) -> RuntimeControlFuture<'a, RuntimeBindingSnapshotV1>;
    fn writer_lease<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeWriterLeaseRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriterLeaseV1>;
    fn report_observation<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeObservationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1>;
    fn register_instance<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &'a RuntimeInstanceRegistrationV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1>;
    fn revoke_instance<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        instance_id: Uuid,
        request: &'a RuntimeInstanceRevokeRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1>;
    fn begin_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        request: &'a RuntimeBeginEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeEffectReservationV1>;
    fn complete_effect<'a>(
        &'a self,
        caller: &'a RuntimeControlCallerV1,
        project_id: Uuid,
        binding_id: Uuid,
        effect_id: Uuid,
        request: &'a RuntimeCompleteEffectRequestV1,
    ) -> RuntimeControlFuture<'a, RuntimeWriteAckV1>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // cyberphone/json-canonicalization testdata, last testdata commit
    // dc406ceaf94b5fa554fcabb92c091089c2357e83 (retrieved 2026-10-20).
    #[test]
    fn jcs_cyberphone_corpus() {
        let vectors = [
            (r#"[56,{"d":true,"10":null,"1":[]}]"#, r#"[56,{"1":[],"10":null,"d":true}]"#),
            (r#"{"peach":"This sorting order","péché":"is wrong according to French","pêche":"but canonicalization MUST","sin":"ignore locale"}"#, r#"{"peach":"This sorting order","péché":"is wrong according to French","pêche":"but canonicalization MUST","sin":"ignore locale"}"#),
            (r#"{"1":{"f":{"f":"hi","F":5},"\n":56.0},"10":{},"":"empty","a":{},"111":[{"e":"yes","E":"no"}],"A":{}}"#, r#"{"":"empty","1":{"\n":56,"f":{"F":5,"f":"hi"}},"10":{},"111":[{"E":"no","e":"yes"}],"A":{},"a":{}}"#),
            (r#"{"Unnormalized Unicode":"A\u030a"}"#, "{\"Unnormalized Unicode\":\"A\u{30a}\"}"),
            (r#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001],"string":"\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/","literals":[null,true,false]}"#, r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#),
            (r#"{"\u20ac":"Euro Sign","\r":"Carriage Return","\u000a":"Newline","1":"One","\u0080":"Control\u007f","\ud83d\ude02":"Smiley","\u00f6":"Latin Small Letter O With Diaeresis","\ufb33":"Hebrew Letter Dalet With Dagesh","</script>":"Browser Challenge"}"#, "{\"\\n\":\"Newline\",\"\\r\":\"Carriage Return\",\"1\":\"One\",\"</script>\":\"Browser Challenge\",\"\u{80}\":\"Control\u{7f}\",\"ö\":\"Latin Small Letter O With Diaeresis\",\"€\":\"Euro Sign\",\"😂\":\"Smiley\",\"דּ\":\"Hebrew Letter Dalet With Dagesh\"}"),
        ];
        for (index, (input, expected)) in vectors.into_iter().enumerate() {
            let value: Value = serde_json::from_str(input).expect("upstream fixture JSON");
            assert_eq!(
                serde_jcs::to_string(&value).expect("JCS"),
                expected,
                "corpus case {index}"
            );
        }
    }

    // RFC 8785 Appendix B, source https://www.rfc-editor.org/rfc/rfc8785.txt.
    // Test the codec itself, independent of our stricter integer-literal profile.
    #[test]
    fn jcs_rfc8785_appendix_b_number_vectors() {
        let vectors: &[(u64, &str)] = &[
            (0x0000000000000000, "0"),
            (0x8000000000000000, "0"),
            (0x0000000000000001, "5e-324"),
            (0x8000000000000001, "-5e-324"),
            (0x7fefffffffffffff, "1.7976931348623157e+308"),
            (0xffefffffffffffff, "-1.7976931348623157e+308"),
            (0x4340000000000000, "9007199254740992"),
            (0xc340000000000000, "-9007199254740992"),
            (0x4430000000000000, "295147905179352830000"),
            (0x44b52d02c7e14af5, "9.999999999999997e+22"),
            (0x44b52d02c7e14af6, "1e+23"),
            (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
            (0x444b1ae4d6e2ef4e, "999999999999999700000"),
            (0x444b1ae4d6e2ef4f, "999999999999999900000"),
            (0x444b1ae4d6e2ef50, "1e+21"),
            (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
            (0x3eb0c6f7a0b5ed8d, "0.000001"),
            (0x41b3de4355555553, "333333333.3333332"),
            (0x41b3de4355555554, "333333333.33333325"),
            (0x41b3de4355555555, "333333333.3333333"),
            (0x41b3de4355555556, "333333333.3333334"),
            (0x41b3de4355555557, "333333333.33333343"),
            (0xbecbf647612f3696, "-0.0000033333333333333333"),
            (0x43143ff3c1cb0959, "1424953923781206.2"),
        ];
        for (bits, expected) in vectors {
            assert_eq!(
                serde_jcs::to_string(&f64::from_bits(*bits)).expect("finite number"),
                *expected,
                "bits={bits:016x}"
            );
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                serde_jcs::to_vec(&value).is_err(),
                "non-finite must not serialize as null"
            );
        }
    }

    #[test]
    fn jcs_utf16_sort_c0_escaping_and_immutable_subtypes() {
        let value = serde_json::json!({"€":1, "\r":2, "דּ":3, "1":4, "😀":5, "\u{80}":6, "ö":7});
        assert_eq!(
            serde_jcs::to_string(&value).expect("JCS"),
            "{\"\\r\":2,\"1\":4,\"\u{80}\":6,\"ö\":7,\"€\":1,\"😀\":5,\"דּ\":3}"
        );
        let controls: String = (0_u8..32).map(char::from).collect();
        assert_eq!(
            serde_jcs::to_string(&controls).expect("C0"),
            r#""\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t\n\u000b\f\r\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f""#
        );
        let subtype = serde_json::json!({"time":"2019-01-28T07:45:10Z", "big":"055", "val":3.5});
        assert_eq!(
            serde_jcs::to_string(&subtype).expect("Appendix E"),
            r#"{"big":"055","time":"2019-01-28T07:45:10Z","val":3.5}"#
        );
        assert!(
            parse_runtime_json::<Value>(br#"{"schema_version":1,"payload":{"a":1,"a":2}}"#)
                .is_err()
        );
        assert!(parse_runtime_json::<Value>(
            br#"{"schema_version":1,"payload":184467440737095516160}"#
        )
        .is_err());
        assert!(
            parse_runtime_json::<Value>(br#"{"schema_version":1,"payload":"\ud800"}"#).is_err()
        );
    }

    #[test]
    fn decimal_wire_preserves_full_counter_range_and_rejects_noncanonical() {
        let maximum = RuntimeCounterV1::new(i64::MAX as u64).expect("maximum is valid");
        let json = serde_json::to_string(&maximum).expect("serializable");
        assert_eq!(json, "\"9223372036854775807\"");
        assert_eq!(
            serde_json::from_str::<RuntimeCounterV1>(&json)
                .expect("valid")
                .get(),
            maximum.get()
        );
        for invalid in ["0", "\"00\"", "\"+1\"", "\"-1\"", "\"9223372036854775808\""] {
            assert!(serde_json::from_str::<RuntimeCounterV1>(invalid).is_err());
        }
        assert!(maximum.checked_next().is_err());
    }

    #[test]
    fn unknown_version_unknown_fields_and_missing_nullable_are_rejected() {
        let expected = serde_json::json!({"generation":"1", "revocation_epoch":"1", "projection_version":"1", "grant_revision":"1"});
        let mut body = serde_json::json!({"schema_version":1, "holder_id":Uuid::nil(), "expected":expected, "renew":null});
        assert!(serde_json::from_value::<RuntimeWriterLeaseRequestV1>(body.clone()).is_ok());
        body.as_object_mut().expect("object").remove("renew");
        assert!(serde_json::from_value::<RuntimeWriterLeaseRequestV1>(body.clone()).is_err());
        body["renew"] = Value::Null;
        body["schema_version"] = serde_json::json!(2);
        assert!(serde_json::from_value::<RuntimeWriterLeaseRequestV1>(body.clone()).is_err());
        body["schema_version"] = serde_json::json!(1);
        body["extra"] = Value::Null;
        assert!(serde_json::from_value::<RuntimeWriterLeaseRequestV1>(body).is_err());
    }
}
