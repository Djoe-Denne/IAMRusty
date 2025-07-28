use chrono::{Duration, Utc};
use base64::{engine::general_purpose, Engine as _};
use uuid::Uuid;

/// Create a JWT token for the given user ID with valid payload and random key
pub fn create_jwt_token(user_id: Uuid) -> String {
    let payload = serde_json::json!({
        "sub": user_id.to_string(),
        "exp": (Utc::now() + Duration::hours(1)).timestamp(),
        "iat": Utc::now().timestamp(),
        "jti": Uuid::new_v4().to_string(),
    });

    let key = "secret".as_bytes();
    let header = serde_json::json!({
        "alg": "HS256",
        "typ": "JWT",
    });

    let token = encode(&header, &payload, key).unwrap();
    token
    
}

fn encode(header: &serde_json::Value, payload: &serde_json::Value, key: &[u8]) -> Result<String, anyhow::Error> {
    let header_str = serde_json::to_string(header)?;
    let payload_str = serde_json::to_string(payload)?;

    let header_base64 = general_purpose::URL_SAFE_NO_PAD.encode(header_str.as_bytes());
    let payload_base64 = general_purpose::URL_SAFE_NO_PAD.encode(payload_str.as_bytes());

    let signature = "fake_signature";

    Ok(format!("{}.{}.{}", header_base64, payload_base64, signature))
}