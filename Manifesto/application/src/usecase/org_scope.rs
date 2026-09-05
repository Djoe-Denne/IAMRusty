use async_trait::async_trait;
use uuid::Uuid;

use crate::ApplicationError;

/// OpenFGA-backed org membership used by project list SQL (not `ListObjects`).
#[async_trait]
pub trait OrgScopeLookup: Send + Sync {
    /// Organization IDs where the user is a stored owner, admin, member, or viewer.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the OpenFGA Read call fails.
    async fn org_ids_where_viewer(&self, user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError>;

    /// Organization IDs where the user is a stored owner or admin.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the OpenFGA Read call fails.
    async fn org_ids_where_admin(&self, user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError>;
}

/// Empty lookup used by unit tests that do not boot OpenFGA.
pub struct NoopOrgScopeLookup;

#[async_trait]
impl OrgScopeLookup for NoopOrgScopeLookup {
    async fn org_ids_where_viewer(&self, _user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError> {
        Ok(Vec::new())
    }

    async fn org_ids_where_admin(&self, _user_id: Uuid) -> Result<Vec<Uuid>, ApplicationError> {
        Ok(Vec::new())
    }
}
