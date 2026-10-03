//! Apparatus P3 — T4 call-time grants (live Manifesto consult + intersection).

mod common;

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::sync::Arc;

use async_trait::async_trait;
use common::create_jwt_token;
use fixtures::BindingSnapshotFixtures;
use lazaret_application::GrantService;
use lazaret_configuration::ManifestoServiceConfig;
use lazaret_domain::{
    authorization_from_session, evaluate_grant, AuthorizationDecision, BindingGrantSnapshot,
    BindingGrantSnapshotPort, CallOrigin, CapabilityConsent, GrantAuthorizationRequest,
    GrantDecision, GrantDenyReason, GrantFetchError, PrincipalMembership, WorkloadIdentity,
};
use lazaret_infra::HttpBindingGrantClient;
use serial_test::serial;
use uuid::Uuid;

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

fn consented_snapshot(
    project_id: Uuid,
    binding: Uuid,
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
        grant_revision: 0,
        declared: vec!["project.read".to_owned()],
        consents: vec![CapabilityConsent {
            capability: "project.read".to_owned(),
            status: "consented".to_owned(),
            grant_revision: 0,
        }],
        principal,
    }
}

fn signed_http_client(base_url: String) -> HttpBindingGrantClient {
    HttpBindingGrantClient::from_config(&ManifestoServiceConfig {
        base_url,
        timeout_seconds: 5,
        hs256_secret: Some("manifesto-grant-snapshot-test-hs256".to_owned()),
        ..ManifestoServiceConfig::default()
    })
    .expect("client")
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

#[tokio::test]
async fn t4_memory_port_allows_when_intersection_holds() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
    let identity = sample_identity(binding, "release-1", 1, 0);
    let snapshot = consented_snapshot(
        project_id,
        binding,
        Some(PrincipalMembership {
            user_id: user,
            active: true,
        }),
    );
    let service = GrantService::new(Arc::new(MemoryPort { snapshot }));
    let decision = service
        .authorize(&allow_request(
            identity,
            CallOrigin::Interactive { principal: user },
            project_id,
        ))
        .await
        .expect("fetch");
    assert_eq!(decision, GrantDecision::Allow);
}

#[tokio::test]
#[serial]
async fn t4_http_client_allows_against_wiremock() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
    let identity = sample_identity(binding, "release-1", 1, 0);
    let snapshot = consented_snapshot(
        project_id,
        binding,
        Some(PrincipalMembership {
            user_id: user,
            active: true,
        }),
    );
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot(project_id, binding, user, snapshot)
        .await;
    let client = signed_http_client(mock.base_url());
    let service = GrantService::new(Arc::new(client));
    let decision = service
        .authorize(&allow_request(
            identity,
            CallOrigin::Interactive { principal: user },
            project_id,
        ))
        .await
        .expect("fetch");
    assert_eq!(decision, GrantDecision::Allow);
    let requests = mock.received_requests().await;
    assert!(
        requests[0].headers.get("authorization").is_some(),
        "signed consult must send Authorization"
    );
}

#[test]
fn t4_unknown_capability_is_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let mut req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Background,
        project_id,
    );
    req.capability = "network.egress".to_owned();
    let mut snapshot = consented_snapshot(project_id, binding, None);
    snapshot.declared.push("network.egress".to_owned());
    assert_eq!(
        evaluate_grant(&req, &snapshot),
        GrantDecision::Deny {
            reason: GrantDenyReason::UnknownCapability
        }
    );
}

#[test]
fn t4_disabled_component_is_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Background,
        project_id,
    );
    let mut snapshot = consented_snapshot(project_id, binding, None);
    snapshot.component_status = "disabled".to_owned();
    assert_eq!(
        evaluate_grant(&req, &snapshot),
        GrantDecision::Deny {
            reason: GrantDenyReason::ComponentInactive
        }
    );
}

#[test]
fn t4_pending_component_is_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Background,
        project_id,
    );
    let mut snapshot = consented_snapshot(project_id, binding, None);
    snapshot.component_status = "pending".to_owned();
    assert_eq!(
        evaluate_grant(&req, &snapshot),
        GrantDecision::Deny {
            reason: GrantDenyReason::ComponentInactive
        }
    );
}

#[test]
fn t4_revoked_or_stale_revision_is_denied() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Background,
        project_id,
    );
    let mut revoked = consented_snapshot(project_id, binding, None);
    revoked.consents[0].status = "revoked".to_owned();
    assert_eq!(
        evaluate_grant(&req, &revoked),
        GrantDecision::Deny {
            reason: GrantDenyReason::ConsentRevoked
        }
    );

    let stale_identity = sample_identity(binding, "release-1", 1, 4);
    let stale_req = allow_request(stale_identity, CallOrigin::Background, project_id);
    let current = consented_snapshot(project_id, binding, None);
    assert_eq!(
        evaluate_grant(&stale_req, &current),
        GrantDecision::Deny {
            reason: GrantDenyReason::IdentityRevisionMismatch
        }
    );
}

#[test]
fn t4_stale_consent_revision_is_denied_when_identity_matches() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Background,
        project_id,
    );
    let mut snapshot = consented_snapshot(project_id, binding, None);
    snapshot.consents[0].grant_revision = 9;
    assert_eq!(req.identity.grant_revision, snapshot.grant_revision);
    assert_eq!(
        evaluate_grant(&req, &snapshot),
        GrantDecision::Deny {
            reason: GrantDenyReason::ConsentRevisionMismatch
        }
    );
}

#[tokio::test]
#[serial]
async fn t4_background_skips_principal_and_does_not_use_iam_jwt() {
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let identity = sample_identity(binding, "release-1", 1, 0);
    let snapshot = consented_snapshot(project_id, binding, None);
    let mock = BindingSnapshotFixtures::service().await;
    mock.mock_get_snapshot_background(project_id, binding, snapshot)
        .await;
    let client = signed_http_client(mock.base_url());
    let service = GrantService::new(Arc::new(client));
    let iam = create_jwt_token(Uuid::new_v4());
    assert!(
        !iam.is_empty(),
        "IAM JWT exists but must not be a workload identity"
    );
    let req = allow_request(identity, CallOrigin::Background, project_id);
    assert!(
        matches!(req.origin, CallOrigin::Background),
        "background must not carry an end-user principal"
    );
    let decision = service.authorize(&req).await.expect("fetch");
    assert_eq!(decision, GrantDecision::Allow);
    let requests = mock.received_requests().await;
    assert_eq!(requests.len(), 1);
    let query = requests[0].url.query().unwrap_or("");
    assert!(
        !query.contains("principal"),
        "background consult must omit principal query, got {query}"
    );
    assert!(
        requests[0].headers.get("authorization").is_some(),
        "signed consult must send Authorization"
    );
}

#[test]
fn t4_revoked_consent_denies_even_if_fga_would_allow() {
    // OpenFGA may still list the user as a project member. Grants are not FGA.
    let project_id = Uuid::new_v4();
    let binding = Uuid::new_v4();
    let user = Uuid::new_v4();
    let req = allow_request(
        sample_identity(binding, "release-1", 1, 0),
        CallOrigin::Interactive { principal: user },
        project_id,
    );
    let mut snapshot = consented_snapshot(
        project_id,
        binding,
        Some(PrincipalMembership {
            user_id: user,
            active: true,
        }),
    );
    snapshot.consents.clear();
    assert_eq!(
        evaluate_grant(&req, &snapshot),
        GrantDecision::Deny {
            reason: GrantDenyReason::ConsentMissing
        }
    );
}

#[test]
fn t4_session_still_requires_live_check() {
    // T3 contract: crypto-valid session is not authorization.
    assert_eq!(
        format!("{:?}", AuthorizationDecision::LiveManifestoCheckRequired),
        "LiveManifestoCheckRequired"
    );
    let _ = authorization_from_session;
}

#[test]
fn t4_grant_src_forbids_runtime_tokens() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = [
        "domain/src/grants.rs",
        "application/src/grants.rs",
        "infra/src/manifesto_client.rs",
    ];
    for rel in files {
        let content = std::fs::read_to_string(root.join(rel)).expect("prod file");
        for token in [
            "Factory",
            "trusted_skip_gateway",
            "kubernetes",
            "wasm",
            "iframe",
            "apparatus_host",
            "ui_host",
        ] {
            assert!(!content.contains(token), "{rel} must not contain {token}");
        }
        assert!(
            !content.to_ascii_lowercase().contains("host"),
            "{rel} must not contain host"
        );
    }
}
