//! Fixed vendor HTTP limits, shared with the setup client builder.

use std::time::Duration;

/// Maximum duration of one complete vendor HTTP request, including body reading.
pub const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum time allowed to establish a vendor connection.
pub const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum decompressed response body, enforced before JSON deserialization.
pub const HTTP_BODY_LIMIT: usize = 1024 * 1024;
