//! InProcess Lazaret → Manifesto binding-grant adapter (ADR-0104).
//!
//! Lives only in `oodhive-monolith`. Maps Manifesto reader DTOs → Lazaret
//! domain snapshots. No HTTP, no JWT `manifesto-bindings`, no token.

use std::sync::Arc;

use async_trait::async_trait;
use lazaret_domain::{
    BindingGrantSnapshot, BindingGrantSnapshotPort, CapabilityConsent, GrantFetchError,
    PrincipalMembership,
};
use manifesto_application::{
    ApplicationError, BindingGrantSnapshotReader, BindingGrantSnapshotResponse,
    CapabilityConsentSnapshot, PrincipalMembershipSnapshot,
};
use rustycog::core::error::DomainError;
use uuid::Uuid;

/// Capability injected into Lazaret via `AppBuilder::with_outbound`.
pub struct InProcessBindingGrantClient {
    reader: Arc<dyn BindingGrantSnapshotReader>,
}

impl InProcessBindingGrantClient {
    #[must_use]
    pub fn new(reader: Arc<dyn BindingGrantSnapshotReader>) -> Self {
        Self { reader }
    }
}

#[async_trait]
impl BindingGrantSnapshotPort for InProcessBindingGrantClient {
    async fn fetch(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        let response = self
            .reader
            .load(project_id, component_id, principal)
            .await
            .map_err(map_application_error)?;
        Ok(map_snapshot(response))
    }
}

fn map_snapshot(response: BindingGrantSnapshotResponse) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: response.component_id,
        project_id: response.project_id,
        project_status: response.project_status,
        component_status: response.component_status,
        source: response.source,
        digest: response.digest,
        desired_generation: response.desired_generation,
        observed_generation: response.observed_generation,
        grant_revision: response.grant_revision,
        declared: response.declared,
        consents: response.consents.into_iter().map(map_consent).collect(),
        principal: response.principal.map(map_principal),
    }
}

fn map_consent(consent: CapabilityConsentSnapshot) -> CapabilityConsent {
    CapabilityConsent {
        capability: consent.capability,
        status: consent.status,
        grant_revision: consent.grant_revision,
    }
}

fn map_principal(membership: PrincipalMembershipSnapshot) -> PrincipalMembership {
    PrincipalMembership {
        user_id: membership.user_id,
        active: membership.active,
    }
}

fn map_application_error(err: ApplicationError) -> GrantFetchError {
    match err {
        ApplicationError::NotFound(_) => GrantFetchError::NotFound,
        ApplicationError::Domain(DomainError::EntityNotFound { .. }) => GrantFetchError::NotFound,
        other => GrantFetchError::Transport(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct FakeReader {
        snapshot: Option<BindingGrantSnapshotResponse>,
    }

    #[async_trait]
    impl BindingGrantSnapshotReader for FakeReader {
        async fn load(
            &self,
            project_id: Uuid,
            component_id: Uuid,
            principal: Option<Uuid>,
        ) -> Result<BindingGrantSnapshotResponse, ApplicationError> {
            match &self.snapshot {
                Some(snapshot) => {
                    let mut out = snapshot.clone();
                    out.project_id = project_id;
                    out.component_id = component_id;
                    out.principal = principal.map(|user_id| PrincipalMembershipSnapshot {
                        user_id,
                        active: true,
                    });
                    Ok(out)
                }
                None => Err(ApplicationError::NotFound("binding".into())),
            }
        }
    }

    fn sample_response() -> BindingGrantSnapshotResponse {
        BindingGrantSnapshotResponse {
            component_id: Uuid::nil(),
            project_id: Uuid::nil(),
            project_status: "active".into(),
            component_status: "active".into(),
            source: "managed".into(),
            digest: Some("abc".into()),
            desired_generation: 1,
            observed_generation: 1,
            grant_revision: 3,
            declared: vec!["storage.kv.read".into()],
            consents: vec![CapabilityConsentSnapshot {
                capability: "storage.kv.read".into(),
                status: "consented".into(),
                grant_revision: 3,
            }],
            principal: Some(PrincipalMembershipSnapshot {
                user_id: Uuid::nil(),
                active: true,
            }),
        }
    }

    #[tokio::test]
    async fn fetch_maps_snapshot_without_http() {
        let reader = Arc::new(FakeReader {
            snapshot: Some(sample_response()),
        });
        let client = InProcessBindingGrantClient::new(reader);
        let project_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let snapshot = client
            .fetch(project_id, component_id, None)
            .await
            .expect("fetch");
        assert_eq!(snapshot.project_id, project_id);
        assert_eq!(snapshot.component_id, component_id);
        assert_eq!(snapshot.grant_revision, 3);
        assert_eq!(snapshot.consents[0].capability, "storage.kv.read");
        assert_eq!(snapshot.consents[0].status, "consented");
        assert_eq!(snapshot.consents[0].grant_revision, 3);
        assert!(snapshot.principal.is_none());
    }

    #[tokio::test]
    async fn fetch_maps_snapshot_with_principal() {
        let reader = Arc::new(FakeReader {
            snapshot: Some(sample_response()),
        });
        let client = InProcessBindingGrantClient::new(reader);
        let user_id = Uuid::new_v4();
        let snapshot = client
            .fetch(Uuid::new_v4(), Uuid::new_v4(), Some(user_id))
            .await
            .expect("fetch");
        assert_eq!(
            snapshot.principal,
            Some(PrincipalMembership {
                user_id,
                active: true,
            })
        );
    }

    #[tokio::test]
    async fn fetch_maps_not_found() {
        let client = InProcessBindingGrantClient::new(Arc::new(FakeReader { snapshot: None }));
        let err = client
            .fetch(Uuid::new_v4(), Uuid::new_v4(), None)
            .await
            .expect_err("not found");
        assert_eq!(err, GrantFetchError::NotFound);
    }

    #[test]
    fn map_domain_entity_not_found() {
        let err = map_application_error(ApplicationError::Domain(DomainError::entity_not_found(
            "binding", "missing",
        )));
        assert_eq!(err, GrantFetchError::NotFound);
    }

    #[test]
    fn map_validation_error_to_transport() {
        let err = map_application_error(ApplicationError::Validation("bad".into()));
        assert!(matches!(err, GrantFetchError::Transport(_)));
        assert_ne!(err, GrantFetchError::NotFound);
    }
}
