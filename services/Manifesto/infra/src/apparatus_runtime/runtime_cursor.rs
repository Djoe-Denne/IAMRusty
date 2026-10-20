//! Authenticated, bounded pagination cursor. Never an authorization cache.

use apparatus_contracts::runtime::{
    parse_runtime_json, RuntimeCursorV1, RuntimeErrorCodeV1, RuntimeInstanceCursorPayloadV1,
    RuntimeInstanceCursorV1, RuntimeListCursorPayloadV1, RuntimeServiceRoleV1,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

const DOMAIN: &[u8] = b"urn:aiforall:apparatus:runtime:cursor:v1";
const INSTANCE_DOMAIN: &[u8] = b"urn:aiforall:apparatus:runtime:instance-cursor:v1";

/// One dedicated key loaded by reference from trusted secret configuration.
/// Deliberately no Debug/Serialize implementation.
pub struct RuntimeCursorKey {
    kid: String,
    key: [u8; 32],
}

impl RuntimeCursorKey {
    /// Encode an inventory-only instance cursor in its separate MAC domain.
    ///
    /// # Errors
    /// Rejects invalid payload bounds or oversized cursor output.
    pub fn encode_instance(
        &self,
        payload: &RuntimeInstanceCursorPayloadV1,
    ) -> Result<RuntimeInstanceCursorV1, RuntimeErrorCodeV1> {
        let encoded = URL_SAFE_NO_PAD.encode(payload.canonical_bytes()?);
        let mac = self.mac_domain(INSTANCE_DOMAIN, &encoded)?;
        let cursor = format!(
            "rci1.{}.{encoded}.{}",
            self.kid,
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        );
        if cursor.len() > 2048 {
            return Err(RuntimeErrorCodeV1::RuntimeCursorInvalid);
        }
        Ok(RuntimeInstanceCursorV1(cursor))
    }

    /// Verify an instance cursor, including its binding and separate MAC domain.
    ///
    /// # Errors
    /// Rejects tampering/cross-scope use, changed page size or absolute expiration.
    pub fn decode_instance(
        &self,
        cursor: &RuntimeInstanceCursorV1,
        role: RuntimeServiceRoleV1,
        project_id: Uuid,
        binding_id: Uuid,
        page_size: Option<u16>,
        now_ms: i64,
    ) -> Result<RuntimeInstanceCursorPayloadV1, RuntimeErrorCodeV1> {
        let invalid = RuntimeErrorCodeV1::RuntimeCursorInvalid;
        if cursor.0.len() > 2048 || !cursor.0.is_ascii() {
            return Err(invalid);
        }
        let mut parts = cursor.0.split('.');
        if parts.next() != Some("rci1") || parts.next() != Some(self.kid.as_str()) {
            return Err(invalid);
        }
        let encoded = parts.next().ok_or(invalid)?;
        let signature = decode_canonical(parts.next().ok_or(invalid)?)?;
        if parts.next().is_some() {
            return Err(invalid);
        }
        self.mac_domain(INSTANCE_DOMAIN, encoded)?
            .verify_slice(&signature)
            .map_err(|_| invalid)?;
        let bytes = decode_canonical(encoded)?;
        let payload: RuntimeInstanceCursorPayloadV1 =
            parse_runtime_json(&bytes).map_err(|_| invalid)?;
        if payload.canonical_bytes()? != bytes
            || payload.project_id != project_id
            || payload.binding_id != binding_id
            || payload.principal != role.principal()
            || page_size.is_some_and(|size| size != payload.page_size)
            || now_ms < payload.issued_at_unix_ms.get()
        {
            return Err(invalid);
        }
        if now_ms >= payload.expires_at_unix_ms.get() {
            return Err(RuntimeErrorCodeV1::RuntimeCursorExpired);
        }
        Ok(payload)
    }
    /// Validate a canonical versioned UUID-v4 key identifier.
    ///
    /// # Errors
    /// Rejects an invalid/noncanonical key id. Key bytes are never returned.
    pub fn new(kid: String, key: [u8; 32]) -> Result<Self, RuntimeErrorCodeV1> {
        let suffix = kid
            .strip_prefix("rcl-v1-")
            .ok_or(RuntimeErrorCodeV1::RuntimeKeyUnavailable)?;
        let uuid =
            Uuid::parse_str(suffix).map_err(|_| RuntimeErrorCodeV1::RuntimeKeyUnavailable)?;
        if uuid.get_version_num() != 4 || uuid.to_string() != suffix {
            return Err(RuntimeErrorCodeV1::RuntimeKeyUnavailable);
        }
        Ok(Self { kid, key })
    }

    /// Encode the exact cursor with a dedicated HMAC key.
    ///
    /// # Errors
    /// Rejects malformed payloads or cursors larger than 2048 bytes.
    pub fn encode(
        &self,
        payload: &RuntimeListCursorPayloadV1,
    ) -> Result<RuntimeCursorV1, RuntimeErrorCodeV1> {
        let encoded = URL_SAFE_NO_PAD.encode(payload.canonical_bytes()?);
        let mac = self.mac(&encoded)?;
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        let cursor = format!("rcl1.{}.{encoded}.{signature}", self.kid);
        if cursor.len() > 2048 {
            return Err(RuntimeErrorCodeV1::RuntimeCursorInvalid);
        }
        Ok(RuntimeCursorV1(cursor))
    }

    /// Verify MAC in constant time before parsing cursor data. Caller/time/project
    /// are authoritative inputs, not taken from headers or the cursor itself.
    ///
    /// # Errors
    /// Rejects unknown keys, noncanonical encodings, tampering, cross-project/
    /// principal use, page-size changes and expiration without grace.
    pub fn decode(
        &self,
        cursor: &RuntimeCursorV1,
        role: RuntimeServiceRoleV1,
        project_id: Uuid,
        page_size: Option<u16>,
        now_ms: i64,
    ) -> Result<RuntimeListCursorPayloadV1, RuntimeErrorCodeV1> {
        let invalid = RuntimeErrorCodeV1::RuntimeCursorInvalid;
        if cursor.0.len() > 2048 || !cursor.0.is_ascii() {
            return Err(invalid);
        }
        let mut parts = cursor.0.split('.');
        if parts.next() != Some("rcl1") || parts.next() != Some(self.kid.as_str()) {
            return Err(invalid);
        }
        let encoded = parts.next().ok_or(invalid)?;
        let encoded_mac = parts.next().ok_or(invalid)?;
        if parts.next().is_some() {
            return Err(invalid);
        }
        let signature = decode_canonical(encoded_mac)?;
        self.mac(encoded)?
            .verify_slice(&signature)
            .map_err(|_| invalid)?;
        let bytes = decode_canonical(encoded)?;
        let payload: RuntimeListCursorPayloadV1 =
            parse_runtime_json(&bytes).map_err(|_| invalid)?;
        if payload.canonical_bytes()? != bytes
            || payload.project_id != project_id
            || payload.principal != role.principal()
            || page_size.is_some_and(|size| size != payload.page_size)
            || now_ms < payload.issued_at_unix_ms.get()
        {
            return Err(invalid);
        }
        if now_ms >= payload.expires_at_unix_ms.get() {
            return Err(RuntimeErrorCodeV1::RuntimeCursorExpired);
        }
        Ok(payload)
    }

    fn mac(&self, encoded: &str) -> Result<Hmac<Sha256>, RuntimeErrorCodeV1> {
        self.mac_domain(DOMAIN, encoded)
    }

    fn mac_domain(&self, domain: &[u8], encoded: &str) -> Result<Hmac<Sha256>, RuntimeErrorCodeV1> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key)
            .map_err(|_| RuntimeErrorCodeV1::RuntimeKeyUnavailable)?;
        mac.update(domain);
        mac.update(&[0]);
        mac.update(self.kid.as_bytes());
        mac.update(&[0]);
        mac.update(encoded.as_bytes());
        Ok(mac)
    }
}

fn decode_canonical(encoded: &str) -> Result<Vec<u8>, RuntimeErrorCodeV1> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| RuntimeErrorCodeV1::RuntimeCursorInvalid)?;
    if URL_SAFE_NO_PAD.encode(&bytes) != encoded {
        return Err(RuntimeErrorCodeV1::RuntimeCursorInvalid);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apparatus_contracts::runtime::RuntimeUnixMsV1;

    #[test]
    fn cursor_is_bound_to_principal_project_page_size_and_absolute_expiry() {
        let key =
            RuntimeCursorKey::new(format!("rcl-v1-{}", Uuid::new_v4()), [7; 32]).expect("key");
        let project_id = Uuid::new_v4();
        let payload = RuntimeListCursorPayloadV1 {
            schema_version: 1,
            scan_id: Uuid::new_v4(),
            project_id,
            principal: RuntimeServiceRoleV1::Projector.principal().to_owned(),
            after_binding_id: Uuid::nil(),
            upper_binding_id: Uuid::from_u128(u128::MAX),
            page_size: 100,
            issued_at_unix_ms: RuntimeUnixMsV1::new(1000).expect("time"),
            expires_at_unix_ms: RuntimeUnixMsV1::new(61000).expect("time"),
        };
        let cursor = key.encode(&payload).expect("encode");
        let decoded = key
            .decode(
                &cursor,
                RuntimeServiceRoleV1::Projector,
                project_id,
                Some(100),
                60999,
            )
            .expect("decode");
        assert_eq!(decoded.scan_id, payload.scan_id);
        assert_eq!(
            key.decode(
                &cursor,
                RuntimeServiceRoleV1::Projector,
                project_id,
                None,
                61000
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorExpired)
        );
        assert_eq!(
            key.decode(
                &cursor,
                RuntimeServiceRoleV1::Lazaret,
                project_id,
                None,
                2000
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
        assert_eq!(
            key.decode(
                &cursor,
                RuntimeServiceRoleV1::Projector,
                Uuid::new_v4(),
                None,
                2000
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
        assert_eq!(
            key.decode(
                &cursor,
                RuntimeServiceRoleV1::Projector,
                project_id,
                Some(200),
                2000
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
        let wrong_key = RuntimeCursorKey::new(key.kid.clone(), [8; 32]).expect("key");
        assert_eq!(
            wrong_key
                .decode(
                    &cursor,
                    RuntimeServiceRoleV1::Projector,
                    project_id,
                    None,
                    2000
                )
                .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
        let padded = RuntimeCursorV1(format!("{}=", cursor.0));
        assert_eq!(
            key.decode(
                &padded,
                RuntimeServiceRoleV1::Projector,
                project_id,
                None,
                2000
            )
            .err(),
            Some(RuntimeErrorCodeV1::RuntimeCursorInvalid)
        );
    }
}
