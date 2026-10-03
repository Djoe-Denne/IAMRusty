use chrono::Utc;
use hive_domain::{
    ExternalLink, ExternalProvider, InvitationStatus, Organization, OrganizationInvitation,
    Permission, PermissionLevel, PermissionResourceCombo, Resource, RolePermission, SyncHealth,
    SyncStatus,
};
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn organization_mutations_and_validation() {
    let owner_id = Uuid::new_v4();
    let mut org = Organization::new(
        "Hive Org".to_string(),
        "hive-org".to_string(),
        Some("desc".to_string()),
        owner_id,
    )
    .unwrap();
    assert!(org.is_owned_by(&owner_id));
    org.update_name("Renamed".to_string()).unwrap();
    org.update_description(Some("new".to_string()));
    org.update_description(Some("x".repeat(1001)));
    org.update_avatar_url(Some("https://example.test/a.png".to_string()));
    org.update_settings(serde_json::json!({"visibility": "Public"}));
    assert!(Organization::new(String::new(), "slug".to_string(), None, owner_id).is_err());
    assert!(Organization::new("n".to_string(), String::new(), None, owner_id).is_err());
    assert!(Organization::new("n".repeat(256), "slug".to_string(), None, owner_id).is_err());
    assert!(Organization::new("n".to_string(), "s".repeat(101), None, owner_id).is_err());
}

#[test]
fn invitation_status_machine() {
    let org_id = Uuid::new_v4();
    let permission = Permission::new(PermissionLevel::Read, None, None);
    let resource = Resource::from("organization".to_string());
    let role = RolePermission::new(
        None,
        Some("reader".to_string()),
        org_id,
        &permission,
        &resource,
        Some(Utc::now()),
    );
    let mut invitation = OrganizationInvitation::new(
        org_id,
        "invitee@example.test".to_string(),
        vec![role.clone()],
        Uuid::new_v4(),
        Some("welcome".to_string()),
    )
    .unwrap();
    invitation.update_organization_name("Hive Org");
    assert!(invitation.is_valid());
    assert!(invitation.can_be_accepted());
    invitation.accept().unwrap();
    assert_eq!(invitation.status, InvitationStatus::Accepted);
    assert!(invitation.accept().is_err());
    assert!(invitation.cancel().is_err());

    let mut pending = OrganizationInvitation::new(
        org_id,
        "second@example.test".to_string(),
        vec![role],
        Uuid::new_v4(),
        None,
    )
    .unwrap();
    pending.cancel().unwrap();
    assert_eq!(invitation.status.as_str(), "accepted");
    assert_eq!(InvitationStatus::Pending.as_str(), "pending");
    assert_eq!(InvitationStatus::Expired.as_str(), "expired");
    assert_eq!(InvitationStatus::Cancelled.as_str(), "cancelled");

    pending.status = InvitationStatus::Expired;
    assert!(pending.accept().is_err());
    assert!(pending.cancel().is_err());
    pending.status = InvitationStatus::Cancelled;
    assert!(pending.accept().is_err());
    pending.mark_expired();
    pending.expires_at = Utc::now() - chrono::Duration::days(1);
    pending.status = InvitationStatus::Pending;
    pending.mark_expired();
    assert_eq!(pending.status, InvitationStatus::Expired);
    let mut combo = PermissionResourceCombo::new("read".to_string(), "organization".to_string());
    combo.permission_level = "write".to_string();
    let mut named = RolePermission::new(None, None, org_id, &permission, &resource, None);
    named.update_name("writer".to_string());
    assert_eq!(named.name.as_deref(), Some("writer"));
}

#[test]
fn external_provider_and_link_sync_states() {
    let mut provider = ExternalProvider::new(
        "github".to_string(),
        "GitHub".to_string(),
        Some(serde_json::json!({"type": "object"})),
    )
    .unwrap();
    provider.update_name("GitHub Cloud".to_string()).unwrap();
    provider.update_config_schema(None);
    provider.deactivate();
    provider.activate();
    assert!(ExternalProvider::new("github".to_string(), String::new(), None).is_err());
    assert!(ExternalProvider::new("github".to_string(), "x".repeat(101), None).is_err());

    let mut link = ExternalLink::new(
        Uuid::new_v4(),
        Some("Hive Org".to_string()),
        provider.id,
        Some("github".to_string()),
        serde_json::json!({"org": "acme"}),
        None,
    )
    .unwrap();
    link.update_provider_config(serde_json::json!({"org": "acme", "team": "core"}))
        .unwrap();
    assert!(link.update_provider_config(serde_json::json!([])).is_err());
    assert!(link.update_provider_config(serde_json::json!({})).is_err());
    link.update_sync_settings(serde_json::json!({"interval": 60}));
    link.enable_sync();
    assert!(link.is_sync_enabled());
    link.record_sync_success();
    assert!(link.is_last_sync_successful());
    assert_eq!(link.get_sync_health(), SyncHealth::Healthy);
    link.record_sync_partial(Some("partial".to_string()));
    assert_eq!(link.get_sync_health(), SyncHealth::Warning);
    link.record_sync_failure("down".to_string());
    assert_eq!(link.get_sync_health(), SyncHealth::Error);
    link.disable_sync();
    link.set_organization_name("Org".to_string());
    link.set_provider_source("gitlab".to_string());
    assert!(link.has_been_synced());
    assert_eq!(SyncStatus::Success.as_str(), "success");
    assert_eq!(SyncStatus::Failed.as_str(), "failed");
    assert_eq!(SyncStatus::Partial.as_str(), "partial");
    assert_eq!(
        SyncStatus::from_str("success").unwrap(),
        SyncStatus::Success
    );
    assert_eq!(SyncStatus::from_str("FAILED").unwrap(), SyncStatus::Failed);
    assert_eq!(
        SyncStatus::from_str("partial").unwrap(),
        SyncStatus::Partial
    );
    assert!(SyncStatus::from_str("nope").is_err());

    let unsynced = ExternalLink::new(
        Uuid::new_v4(),
        None,
        provider.id,
        None,
        serde_json::json!({"org": "acme"}),
        Some(serde_json::json!({"cron": true})),
    )
    .unwrap();
    assert_eq!(unsynced.get_sync_health(), SyncHealth::Unknown);

    assert_eq!(
        PermissionLevel::from_str("read").unwrap(),
        PermissionLevel::Read
    );
    assert_eq!(PermissionLevel::Write.to_str(), "write");
    assert_eq!(PermissionLevel::Admin.to_str(), "admin");
    assert_eq!(PermissionLevel::Owner.to_str(), "owner");
    assert!(PermissionLevel::from_str("nope").is_err());
    let cog: rustycog::permission::Permission = PermissionLevel::Admin.into();
    let back = PermissionLevel::from(cog);
    assert_eq!(back, PermissionLevel::Admin);
}

#[test]
fn sync_job_progress_and_status_machine() {
    use hive_domain::{SyncJob, SyncJobStatus, SyncJobType};
    use std::str::FromStr;

    let mut job = SyncJob::new(Uuid::new_v4(), SyncJobType::FullSync, None);
    assert!(job.is_running());
    job.update_progress(4, 2, 1, 1).unwrap();
    job.add_progress(1, 1, 0, 0).unwrap();
    job.update_details(serde_json::json!({"step": "members"}))
        .unwrap();
    assert_eq!(job.get_success_rate(), 0.8);
    let summary = job.get_summary();
    assert_eq!(summary.total_processed, 5);
    job.complete_successfully().unwrap();
    assert!(job.is_completed());
    assert!(job.is_finished());
    assert!(job.get_duration().is_some());
    assert!(job.complete_successfully().is_err());
    assert!(job.fail("late".to_string()).is_err());
    assert!(job.update_progress(1, 0, 0, 0).is_err());
    assert!(job.add_progress(1, 0, 0, 0).is_err());
    assert!(job.update_details(serde_json::json!({})).is_err());

    let mut failed = SyncJob::new(
        Uuid::new_v4(),
        SyncJobType::MembersOnly,
        Some(serde_json::json!({"scope": "members"})),
    );
    failed.fail("provider down".to_string()).unwrap();
    assert!(failed.is_failed());
    assert!(failed.fail("again".to_string()).is_err());
    assert!(failed.complete_successfully().is_err());
    assert_eq!(failed.get_success_rate(), 1.0);

    assert_eq!(SyncJobType::FullSync.as_str(), "full_sync");
    assert_eq!(SyncJobType::IncrementalSync.as_str(), "incremental_sync");
    assert_eq!(SyncJobType::MembersOnly.as_str(), "members_only");
    assert_eq!(
        SyncJobType::from_str("full_sync").unwrap(),
        SyncJobType::FullSync
    );
    assert_eq!(
        SyncJobType::from_str("incremental_sync").unwrap(),
        SyncJobType::IncrementalSync
    );
    assert_eq!(
        SyncJobType::from_str("members_only").unwrap(),
        SyncJobType::MembersOnly
    );
    assert!(SyncJobType::from_str("nope").is_err());
    assert_eq!(SyncJobStatus::Running.as_str(), "running");
    assert_eq!(SyncJobStatus::Completed.as_str(), "completed");
    assert_eq!(SyncJobStatus::Failed.as_str(), "failed");
    assert_eq!(
        SyncJobStatus::from_str("running").unwrap(),
        SyncJobStatus::Running
    );
    assert_eq!(
        SyncJobStatus::from_str("COMPLETED").unwrap(),
        SyncJobStatus::Completed
    );
    assert_eq!(
        SyncJobStatus::from_str("failed").unwrap(),
        SyncJobStatus::Failed
    );
    assert!(SyncJobStatus::from_str("nope").is_err());
}
