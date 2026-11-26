//! Authoritative primary-store session-liveness adapter for IAM.

use async_trait::async_trait;
use rustycog::http::{
    SessionLivenessChecker, SessionLivenessDecision, SessionLivenessError, SessionLivenessOutcome,
    SessionLivenessQuery,
};
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::Arc;

use super::authentication_session::read_epoch;
use super::entity::users::Entity as Users;

/// Reads the current account epoch from the authoritative primary connection.
///
/// The adapter performs no caching and never falls back to a read replica or
/// JWT-only validation.
#[derive(Clone)]
pub struct SeaOrmSessionLivenessChecker {
    db: Arc<DatabaseConnection>,
    platform_issuer: String,
}

impl SeaOrmSessionLivenessChecker {
    /// Construct a checker bound to the primary writer connection.
    #[must_use]
    pub const fn new(db: Arc<DatabaseConnection>, platform_issuer: String) -> Self {
        Self {
            db,
            platform_issuer,
        }
    }
}

#[async_trait]
impl SessionLivenessChecker for SeaOrmSessionLivenessChecker {
    async fn check(
        &self,
        query: &SessionLivenessQuery,
    ) -> Result<SessionLivenessOutcome, SessionLivenessError> {
        if query.issuer != self.platform_issuer {
            return Ok(SessionLivenessOutcome::new(
                SessionLivenessDecision::NotApplicable,
            ));
        }
        let user = Users::find_by_id(query.subject)
            .one(self.db.as_ref())
            .await
            .map_err(|_| SessionLivenessError::unavailable("primary epoch read failed"))?;
        let Some(user) = user else {
            return Ok(SessionLivenessOutcome::new(
                SessionLivenessDecision::Revoked,
            ));
        };
        let current_epoch = read_epoch(&user.auth_epoch)
            .map_err(|_| SessionLivenessError::unavailable("stored epoch is invalid"))?;
        if query.auth_epoch == Some(current_epoch)
            || (query.auth_epoch.is_none() && current_epoch == 0)
        {
            Ok(SessionLivenessOutcome::with_current_epoch(
                SessionLivenessDecision::Live,
                current_epoch,
            ))
        } else {
            Ok(SessionLivenessOutcome::with_current_epoch(
                SessionLivenessDecision::Revoked,
                current_epoch,
            ))
        }
    }
}
