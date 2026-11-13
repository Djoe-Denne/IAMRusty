//! QA coverage90-local B: expired pending invitations must not become accepted.
use chrono::{Duration, Utc};
use hive_domain::{InvitationStatus, OrganizationInvitation};
use rustycog::core::error::DomainError;
use uuid::Uuid;

fn invitation() -> OrganizationInvitation {
    let organization_id = Uuid::new_v4();
    OrganizationInvitation::new(
        organization_id,
        organization_id.to_string(),
        Vec::new(),
        Uuid::new_v4(),
        None,
    )
    .unwrap()
}

#[test]
fn accepting_expired_pending_invitation_marks_expired_without_acceptance() {
    let mut invitation = invitation();
    invitation.expires_at = Utc::now() - Duration::days(1);
    assert!(invitation.is_expired());
    assert!(!invitation.can_be_accepted());
    let result = invitation.accept();
    assert!(matches!(
        result,
        Err(DomainError::BusinessRuleViolation { .. })
    ));
    assert_eq!(invitation.status, InvitationStatus::Expired);
    assert_eq!(invitation.accepted_at, None);
}

#[test]
fn expiry_sweep_preserves_terminal_statuses_and_acceptance_timestamp() {
    for status in [
        InvitationStatus::Accepted,
        InvitationStatus::Cancelled,
        InvitationStatus::Expired,
    ] {
        let mut invitation = invitation();
        if status == InvitationStatus::Accepted {
            invitation.accept().unwrap();
        }
        invitation.status = status.clone();
        invitation.expires_at = Utc::now() - Duration::days(1);
        let accepted_at = invitation.accepted_at;
        invitation.mark_expired();
        assert_eq!(invitation.status, status);
        assert_eq!(invitation.accepted_at, accepted_at);
        assert!(!invitation.can_be_accepted());
    }
}

#[test]
fn expiry_sweep_does_not_expire_pending_invitation_before_deadline() {
    let mut invitation = invitation();
    invitation.expires_at = Utc::now() + Duration::days(1);
    invitation.mark_expired();
    assert_eq!(invitation.status, InvitationStatus::Pending);
    assert!(invitation.can_be_accepted());
    assert_eq!(invitation.accepted_at, None);
}
