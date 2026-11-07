//! Fixed vendor HTTP limits, shared with the setup client builder.

use std::time::Duration;

use idp_connect_contract::FederatedOAuthError;
use reqwest::{header::CONTENT_ENCODING, Response};
use serde_json::Value;

/// Maximum duration of one complete vendor HTTP request, including body reading.
pub const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum time allowed to establish a vendor connection.
pub const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum decompressed response body, enforced before JSON deserialization.
pub const HTTP_BODY_LIMIT: usize = 1024 * 1024;

/// Read only delivered (possibly gzip-decoded) chunks, then parse at EOF.
///
/// # Errors
/// Returns the caller's sanitized phase error on status, encoding, size, stream
/// or JSON failure. The response is dropped immediately rather than drained.
pub(super) async fn read_json(
    mut response: Response,
    error: FederatedOAuthError,
) -> Result<Value, FederatedOAuthError> {
    if !response.status().is_success() {
        return Err(error);
    }
    // The trusted reqwest decoder may already have removed these headers. Check
    // every residual value; never infer or perform a second decompression here.
    for encoding in response.headers().get_all(CONTENT_ENCODING) {
        if !encoding
            .to_str()
            .map_err(|_| error)?
            .eq_ignore_ascii_case("identity")
        {
            return Err(error);
        }
    }
    if response
        .content_length()
        .is_some_and(|size| size > HTTP_BODY_LIMIT as u64)
    {
        return Err(error);
    }
    let mut buffer = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| error)? {
        // buffer.len() stays <= the limit. Check BEFORE extension, including
        // when Content-Length is missing/removed or describes compressed bytes.
        if chunk.len() > HTTP_BODY_LIMIT - buffer.len() {
            return Err(error);
        }
        buffer.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&buffer).map_err(|_| error)
}
