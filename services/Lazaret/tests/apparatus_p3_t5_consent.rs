//! Apparatus P3 — T5 close-at-commit grants (stale revision + inactive principal).

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use async_trait::async_trait;
use fixtures::BindingSnapshotFixtures;
use lazaret_application::GrantService;
use lazaret_domain::{
    evaluate_grant, BindingGrantSnapshot, BindingGrantSnapshotPort, CallOrigin, CapabilityConsent,
    GrantAuthorizationRequest, GrantDecision, GrantDenyReason, GrantFetchError,
    PrincipalMembership, WorkloadIdentity,
};
use lazaret_infra::HttpBindingGrantClient;
use serial_test::serial;
use uuid::Uuid;

fn sample_identity(
    binding: Uuid,
    release: &str,
    generation: i64,
    grant_revision: i64,
) -> WorkloadIdentity {
    WorkloadIdentity::try_new(
        Uuid::new_v4(),
        binding,
        release.to_owned(),
        generation,
        grant_revision,
    )
    .expect("identity")
}

fn snapshot(
    project_id: Uuid,
    binding: Uuid,
    grant_revision: i64,
    status: &str,
    principal: Option<PrincipalMembership>,
) -> BindingGrantSnapshot {
    BindingGrantSnapshot {
        component_id: binding,
        project_id,
        project_status: "draft".to_owned(),
        component_status: "active".to_owned(),
        source: "managed".to_owned(),
        digest: Some("release-1".to_owned()),
        desired_generation: 1,
        observed_generation: 0,
        grant_revision,
        declared: vec!["project.read".to_owned()],
        consents: vec![CapabilityConsent {
            capability: "project.read".to_owned(),
            status: status.to_owned(),
            grant_revision,
        }],
        principal,
    }
}

fn allow_request(
    identity: WorkloadIdentity,
    origin: CallOrigin,
    project_id: Uuid,
) -> GrantAuthorizationRequest {
    GrantAuthorizationRequest {
        identity,
        capability: "project.read".to_owned(),
        declared: vec!["project.read".to_owned()],
        origin,
        project_id,
    }
}

struct MemoryPort {
    snapshot: BindingGrantSnapshot,
}

#[async_trait]
impl BindingGrantSnapshotPort for MemoryPort {
    async fn fetch(
        &self,
        _project_id: Uuid,
        _component_id: Uuid,
        _principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshot, GrantFetchError> {
        Ok(self.snapshot.clone())
    }
}

#[tokio::test]
#[serial]
async fn t5_old_identity_revision_denied_after_snapshot_bump() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
    let old_identity = sample_identity(binding, "release-1", 1, 1);
    let bumped = snapshot(
        project_id,
        binding,
        2,
        "revoked",
        Some(PrincipalMembership {
            user_id: user,
            active: true,
        }),
    );
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot(project_id, binding, user, bumped)
        .await;
    let client = HttpBindingGrantClient::new(mock.base_url(), 5, "").expect("client");
    let service = GrantService::new(Arc::new(client));
    let decision = service
        .authorize(&allow_request(
            old_identity,
            CallOrigin::Interactive { principal: user },
            project_id,
        ))
        .await
        .expect("fetch");
    assert_eq!(
        decision,
        GrantDecision::Deny {
            reason: GrantDenyReason::IdentityRevisionMismatch
        }
    );
}

#[test]
fn t5_inactive_principal_denied_even_if_fga_would_still_allow() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 1),
        CallOrigin::Interactive { principal: user },
        project_id,
    );
    let snap = snapshot(
        project_id,
        binding,
        1,
        "consented",
        Some(PrincipalMembership {
            user_id: user,
            active: false,
        }),
    );
    assert_eq!(
        evaluate_grant(&req, &snap),
        GrantDecision::Deny {
            reason: GrantDenyReason::PrincipalInactive
        }
    );
}

#[tokio::test]
async fn t5_memory_port_denies_revoked_consent_at_new_revision() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = sample_identity(binding, "release-1", 1, 2);
    let snap = snapshot(project_id, binding, 2, "revoked", None);
    let service = GrantService::new(Arc::new(MemoryPort { snapshot: snap }));
    let decision = service
        .authorize(&allow_request(identity, CallOrigin::Background, project_id))
        .await
        .expect("fetch");
    assert_eq!(
        decision,
        GrantDecision::Deny {
            reason: GrantDenyReason::ConsentRevoked
        }
    );
}
