//! Fetch live Manifesto snapshot then evaluate the grant intersection.

use std::sync::Arc;

use lazaret_domain::{
    evaluate_grant, BindingGrantSnapshotPort, CallOrigin, GrantAuthorizationRequest, GrantDecision,
    GrantFetchError,
};

/// Application service: live Manifesto consult then [`evaluate_grant`].
pub struct GrantService {
    snapshots: Arc<dyn BindingGrantSnapshotPort>,
}

impl GrantService {
    /// Wire a snapshot port (HTTP in production, fake in tests).
    #[must_use]
    pub fn new(snapshots: Arc<dyn BindingGrantSnapshotPort>) -> Self {
        Self { snapshots }
    }

    /// Fetch the live snapshot then evaluate the intersection.
    ///
    /// Does not treat a crypto-valid session as authorization.
    ///
    /// # Errors
    ///
    /// Returns [`GrantFetchError`] when the Manifesto consult fails.
    pub async fn authorize(
        &self,
        req: &GrantAuthorizationRequest,
    ) -> Result<GrantDecision, GrantFetchError> {
        let principal = match req.origin {
            CallOrigin::Interactive { principal } => Some(principal),
            CallOrigin::Background => None,
        };
        let snapshot = self
            .snapshots
            .fetch(req.project_id, req.identity.binding, principal)
            .await?;
        Ok(evaluate_grant(req, &snapshot))
    }
}
