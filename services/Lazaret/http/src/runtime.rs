//! Runtime-only HTTP boundary (ADR-0012 §13.1-D).
//!
//! Error mapping is enabled; routes await application-service integration.
//! Legacy error mappings remain unchanged.

use apparatus_contracts::runtime::RuntimeErrorCodeV1;

use super::error::InvokeHttpError;

impl From<RuntimeErrorCodeV1> for InvokeHttpError {
    fn from(code: RuntimeErrorCodeV1) -> Self {
        use RuntimeErrorCodeV1::*;
        // Serialize the shared wire enum, never Debug or a second code catalog.
        let Ok(serde_json::Value::String(wire)) = serde_json::to_value(code) else {
            return Self::Failed;
        };
        let reason = wire.to_ascii_lowercase();
        match code {
            RuntimeInvalidRequest
            | RuntimeUnsupportedVersion
            | RuntimeCursorInvalid
            | RuntimeCursorExpired => Self::BadRequest(reason),
            RuntimeUnauthenticated | RuntimeInvalidProof => Self::Unauthorized,
            RuntimeServiceForbidden
            | RuntimeCallerForbidden
            | RuntimeBindingClosed
            | RuntimeReleaseNotAdmitted
            | RuntimeIdentityRevoked
            | RuntimeHashMismatch
            | RuntimeLegacyDisabled
            | RuntimeBindingNotFound
            | RuntimeStaleState
            | RuntimeFenceConflict
            | RuntimeLeaseLost
            | RuntimeReplay
            | RuntimeIdReused => Self::Forbidden(reason),
            RuntimePayloadTooLarge => Self::PayloadTooLarge,
            RuntimeLeaseHeld
            | RuntimeAuthorityUnavailable
            | RuntimeProjectionUnavailable
            | RuntimeRouteUnavailable
            | RuntimeCapacityExhausted
            | RuntimeKeyUnavailable
            | RuntimeDeadlineExceeded
            | RuntimeAuthorityProtocolError
            | RuntimeEffectOutcomeUnknown => Self::BadGateway(reason),
            RuntimeCounterExhausted | RuntimeInternal => Self::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};

    #[test]
    fn runtime_errors_never_expand_public_statuses_or_reuse_internal_statuses() {
        use RuntimeErrorCodeV1::*;
        let cases = [
            (RuntimeInvalidRequest, 400),
            (RuntimeUnsupportedVersion, 400),
            (RuntimeCursorInvalid, 400),
            (RuntimeCursorExpired, 400),
            (RuntimeUnauthenticated, 401),
            (RuntimeInvalidProof, 401),
            (RuntimeServiceForbidden, 403),
            (RuntimeCallerForbidden, 403),
            (RuntimeBindingClosed, 403),
            (RuntimeReleaseNotAdmitted, 403),
            (RuntimeIdentityRevoked, 403),
            (RuntimeHashMismatch, 403),
            (RuntimeLegacyDisabled, 403),
            (RuntimeBindingNotFound, 403),
            (RuntimeStaleState, 403),
            (RuntimeFenceConflict, 403),
            (RuntimeLeaseLost, 403),
            (RuntimeReplay, 403),
            (RuntimeIdReused, 403),
            (RuntimePayloadTooLarge, 413),
            (RuntimeLeaseHeld, 502),
            (RuntimeAuthorityUnavailable, 502),
            (RuntimeProjectionUnavailable, 502),
            (RuntimeRouteUnavailable, 502),
            (RuntimeCapacityExhausted, 502),
            (RuntimeKeyUnavailable, 502),
            (RuntimeDeadlineExceeded, 502),
            (RuntimeAuthorityProtocolError, 502),
            (RuntimeEffectOutcomeUnknown, 502),
            (RuntimeCounterExhausted, 500),
            (RuntimeInternal, 500),
        ];
        for (code, expected) in cases {
            assert_eq!(
                InvokeHttpError::from(code).into_response().status(),
                StatusCode::from_u16(expected).expect("known status")
            );
        }
        let InvokeHttpError::Forbidden(reason) = InvokeHttpError::from(RuntimeStaleState) else {
            panic!("stale state must be forbidden")
        };
        assert_eq!(reason, "runtime_stale_state");
        let InvokeHttpError::BadGateway(reason) =
            InvokeHttpError::from(RuntimeEffectOutcomeUnknown)
        else {
            panic!("ambiguous effect must be bad gateway")
        };
        assert_eq!(reason, "runtime_effect_outcome_unknown");
    }
}
