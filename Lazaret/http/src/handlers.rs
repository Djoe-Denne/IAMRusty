//! Enrollment and session handlers. No IAM `.authenticated()` middleware.

use std::sync::Arc;

use axum::extract::Extension;
use axum::Json;
use lazaret_application::{EnrollCommand, IdentityService};
use lazaret_domain::{IdentityError, VerifiedClientCertificate, WorkloadIdentity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::IdentityHttpError;

#[derive(Deserialize)]
struct EnrollRequest {
    csr_pem: String,
    instance: Uuid,
    binding: Uuid,
    release: String,
    generation: i64,
    grant_revision: i64,
    project_id: Uuid,
}

#[derive(Serialize)]
pub struct EnrollResponse {
    /// PEM client certificate issued by the platform CA.
    pub certificate_pem: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    /// Dedicated Lazaret session token (not an IAM JWT).
    pub session_token: String,
}

/// `POST /enroll` — CSR from the workload, no IAM auth.
pub async fn enroll(
    Extension(identity): Extension<Arc<IdentityService>>,
    Json(raw): Json<serde_json::Value>,
) -> Result<Json<EnrollResponse>, IdentityHttpError> {
    reject_private_key_fields(&raw)?;
    let req: EnrollRequest =
        serde_json::from_value(raw).map_err(|e| IdentityHttpError::bad_request(e.to_string()))?;
    let identity_fields = WorkloadIdentity::try_new(
        req.instance,
        req.binding,
        req.release,
        req.generation,
        req.grant_revision,
    )?;
    let issued = identity
        .enroll(EnrollCommand {
            csr_pem: req.csr_pem,
            identity: identity_fields,
            project_id: req.project_id,
        })
        .await?;
    Ok(Json(EnrollResponse {
        certificate_pem: issued.pem,
    }))
}

/// `POST /session` — requires `Extension<VerifiedClientCertificate>` (mTLS equivalent).
pub async fn session(
    Extension(identity): Extension<Arc<IdentityService>>,
    cert: Option<Extension<VerifiedClientCertificate>>,
) -> Result<Json<SessionResponse>, IdentityHttpError> {
    let Some(Extension(cert)) = cert else {
        return Err(IdentityHttpError::from(
            IdentityError::MissingClientCertificate,
        ));
    };
    let session_token = identity.issue_session(&cert)?;
    Ok(Json(SessionResponse { session_token }))
}

fn reject_private_key_fields(value: &serde_json::Value) -> Result<(), IdentityHttpError> {
    let Some(obj) = value.as_object() else {
        return Err(IdentityHttpError::bad_request("invalid enroll body"));
    };
    for key in obj.keys() {
        if key.to_ascii_lowercase().contains("private") {
            return Err(IdentityHttpError::from(
                IdentityError::PrivateKeyNotAccepted,
            ));
        }
    }
    Ok(())
}
