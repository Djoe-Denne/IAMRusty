//! Hive event -> `OpenFGA` tuple translation.
//!
//! Maps organization lifecycle and membership events from `hive-events`
//! onto tuples for the `organization` `OpenFGA` type.
//!
//! `OrganizationDeleted` deletes every known organization relation for the
//! owner and each member listed on the event. The payload must carry those
//! IDs — translators never read domain state.

use anyhow::{anyhow, Result};
use hive_events::{HiveDomainEvent, Role};
use rustycog::permission::{PrincipalPolicy, PrincipalRef, Subject};

use super::{Translator, TupleDelta};
use crate::fga_client::Tuple;

pub struct HiveTranslator {
    policy: PrincipalPolicy,
}

impl HiveTranslator {
    #[must_use]
    pub const fn new(policy: PrincipalPolicy) -> Self {
        Self { policy }
    }

    fn subject(&self, issuer: Option<&str>, user_id: uuid::Uuid) -> Result<Subject> {
        let unresolved = || anyhow!("EVENT_PRINCIPAL_UNRESOLVED");
        let principal =
            PrincipalRef::new(issuer.ok_or_else(unresolved)?, user_id).map_err(|_| unresolved())?;
        Subject::from_principal(principal, &self.policy).map_err(|_| unresolved())
    }
}

/// Map a Hive `Role.permission` string onto the relation name the `OpenFGA`
/// `organization` type exposes. Returns `None` for unrecognized permission
/// strings — the translator skips them rather than failing the whole event.
fn role_to_org_relation(permission: &str) -> Option<&'static str> {
    match permission.to_lowercase().as_str() {
        "owner" => Some("owner"),
        "admin" => Some("admin"),
        "write" => Some("member"),
        "read" => Some("viewer"),
        _ => None,
    }
}

/// Build one tuple per role on the organization. Sub-resource roles
/// (member, `external_link`, etc.) collapse to organization-level relations
/// because the old Casbin models treated them as "unidentified resources".
fn role_tuples_for_member(
    organization_id: uuid::Uuid,
    subject: &Subject,
    roles: &[Role],
) -> Vec<Tuple> {
    let mut out = Vec::new();
    for role in roles {
        if let Some(relation) = role_to_org_relation(&role.permission) {
            out.push(Tuple::for_subject(
                "organization",
                organization_id,
                relation,
                subject.clone(),
            ));
        }
    }
    // Every member also gets the bare `member` relation so checks for
    // "is this user in the org?" work even without any explicit role.
    out.push(Tuple::for_subject(
        "organization",
        organization_id,
        "member",
        subject.clone(),
    ));
    out
}

/// Delete every known organization relation for one user.
fn delete_all_org_relations(
    mut delta: TupleDelta,
    organization_id: uuid::Uuid,
    subject: &Subject,
) -> TupleDelta {
    for relation in ["owner", "admin", "member", "viewer"] {
        delta = delta.delete(Tuple::for_subject(
            "organization",
            organization_id,
            relation,
            subject.clone(),
        ));
    }
    delta
}

impl Translator for HiveTranslator {
    fn name(&self) -> &'static str {
        "hive"
    }

    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>> {
        let event: HiveDomainEvent = match serde_json::from_value(raw_event.clone()) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        let delta = match event {
            HiveDomainEvent::OrganizationCreated(evt) => {
                TupleDelta::default().write(Tuple::for_subject(
                    "organization",
                    evt.organization_id,
                    "owner",
                    self.subject(evt.owner_issuer.as_deref(), evt.owner_user_id)?,
                ))
            }

            HiveDomainEvent::MemberJoined(evt) => {
                let subject = self.subject(evt.issuer.as_deref(), evt.user_id)?;
                let mut d = TupleDelta::default();
                for t in role_tuples_for_member(evt.organization_id, &subject, &evt.roles) {
                    d = d.write(t);
                }
                d
            }

            HiveDomainEvent::MemberRemoved(evt) => {
                // Remove both the base membership and any role tuples. We
                // cannot know the exact role set at removal time, so we
                // delete every known organization relation for this user.
                let subject = self.subject(evt.issuer.as_deref(), evt.user_id)?;
                delete_all_org_relations(TupleDelta::default(), evt.organization_id, &subject)
            }

            HiveDomainEvent::MemberRolesUpdated(evt) => {
                let subject = self.subject(evt.issuer.as_deref(), evt.user_id)?;
                // Previous roles are not on the event. Drop every known
                // organization relation, then write the new role set.
                let mut d =
                    delete_all_org_relations(TupleDelta::default(), evt.organization_id, &subject);
                for t in role_tuples_for_member(evt.organization_id, &subject, &evt.roles) {
                    d = d.write(t);
                }
                d
            }

            HiveDomainEvent::OrganizationDeleted(evt) => {
                // Resolve the complete payload before returning any deletion.
                // UUID-only legacy lists never become grants or cross-authority deletes.
                let mut subjects =
                    vec![self.subject(evt.owner_issuer.as_deref(), evt.owner_user_id)?];
                for member in evt
                    .member_principals
                    .ok_or_else(|| anyhow!("EVENT_PRINCIPAL_UNRESOLVED"))?
                {
                    subjects.push(self.subject(Some(&member.issuer), member.user_id)?);
                }
                let mut d = TupleDelta::default();
                for subject in subjects {
                    d = delete_all_org_relations(d, evt.organization_id, &subject);
                }
                d
            }

            // Lifecycle and sync events that do not move any authorization
            // fact in OpenFGA.
            HiveDomainEvent::OrganizationUpdated(_)
            | HiveDomainEvent::MemberInvited(_)
            | HiveDomainEvent::InvitationCreated(_)
            | HiveDomainEvent::InvitationAccepted(_)
            | HiveDomainEvent::InvitationExpired(_)
            | HiveDomainEvent::ExternalLinkCreated(_)
            | HiveDomainEvent::SyncJobStarted(_)
            | HiveDomainEvent::SyncJobCompleted(_) => TupleDelta::default(),
        };

        Ok(Some(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use hive_events::{
        MemberJoinedEvent, MemberRolesUpdatedEvent, OrganizationCreatedEvent,
        OrganizationDeletedEvent, Role,
    };
    use uuid::Uuid;
    const PLATFORM: &str = "https://platform.example/iam";

    fn translator() -> HiveTranslator {
        HiveTranslator::new(PrincipalPolicy::new(PLATFORM, false).unwrap())
    }

    fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
        // Wrap into the HiveDomainEvent tagged form by round-tripping.
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn organization_created_writes_owner_tuple() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut event = OrganizationCreatedEvent::new(
            org_id,
            "ACME".to_string(),
            "acme".to_string(),
            user_id,
            Utc::now(),
        );
        event.owner_issuer = Some(PLATFORM.into());
        let evt = HiveDomainEvent::OrganizationCreated(event);
        let translator = translator();
        let delta = translator.translate(&to_json(evt)).unwrap().unwrap();
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].object_type, "organization");
        assert_eq!(delta.writes[0].relation, "owner");
        assert_eq!(delta.writes[0].user_id, user_id.to_string());
    }

    #[test]
    fn member_joined_emits_role_plus_base_member_tuple() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut event = MemberJoinedEvent::new(
            org_id,
            "ACME".to_string(),
            user_id,
            vec![Role::new("admin".to_string(), "organization".to_string())],
            Utc::now(),
        );
        event.issuer = Some(PLATFORM.into());
        let evt = HiveDomainEvent::MemberJoined(event);
        let translator = translator();
        let delta = translator.translate(&to_json(evt)).unwrap().unwrap();
        let relations: Vec<_> = delta.writes.iter().map(|t| t.relation.as_str()).collect();
        assert!(relations.contains(&"admin"));
        assert!(relations.contains(&"member"));
    }

    #[test]
    fn organization_deleted_deletes_owner_and_member_tuples() {
        let org_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();
        let member_id = Uuid::new_v4();
        let mut event = OrganizationDeletedEvent::new(
            org_id,
            "ACME".to_string(),
            owner_id,
            vec![owner_id, member_id],
            owner_id,
            Utc::now(),
        );
        event.owner_issuer = Some(PLATFORM.into());
        event.member_principals = Some(vec![hive_events::MemberPrincipal {
            issuer: PLATFORM.into(),
            user_id: member_id,
        }]);
        let evt = HiveDomainEvent::OrganizationDeleted(event);
        let delta = translator().translate(&to_json(evt)).unwrap().unwrap();
        assert!(delta.writes.is_empty());
        let owner = owner_id.to_string();
        let member = member_id.to_string();
        let deleted_users: Vec<_> = delta.deletes.iter().map(|t| t.user_id.as_str()).collect();
        assert!(deleted_users.contains(&owner.as_str()));
        assert!(deleted_users.contains(&member.as_str()));
        assert!(delta
            .deletes
            .iter()
            .all(|t| t.object_type == "organization"));
    }

    #[test]
    fn member_roles_updated_replaces_role_tuples() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut event = MemberRolesUpdatedEvent::new(
            org_id,
            "ACME".to_string(),
            user_id,
            vec![Role::new("read".to_string(), "organization".to_string())],
            Utc::now(),
        );
        event.issuer = Some(PLATFORM.into());
        let evt = HiveDomainEvent::MemberRolesUpdated(event);
        let delta = translator().translate(&to_json(evt)).unwrap().unwrap();
        let deleted: Vec<_> = delta.deletes.iter().map(|t| t.relation.as_str()).collect();
        let written: Vec<_> = delta.writes.iter().map(|t| t.relation.as_str()).collect();
        assert!(deleted.contains(&"owner"));
        assert!(deleted.contains(&"admin"));
        assert!(deleted.contains(&"member"));
        assert!(deleted.contains(&"viewer"));
        assert!(written.contains(&"viewer"));
        assert!(written.contains(&"member"));
    }

    #[test]
    fn hive_membership_and_projection_preserve_target_issuer() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let mut evt = MemberJoinedEvent::new(
            org_id,
            "ACME".to_string(),
            user_id,
            vec![Role::new("write".to_string(), "organization".to_string())],
            Utc::now(),
        );
        let target = "https://unrelated.example/iam";
        evt.issuer = Some(target.into());
        let expected = translator()
            .subject(Some(target), user_id)
            .unwrap()
            .to_string();
        let delta = translator()
            .translate(&to_json(HiveDomainEvent::MemberJoined(evt.clone())))
            .unwrap()
            .unwrap();
        assert!(delta
            .writes
            .iter()
            .all(|t| { format!("{}:{}", t.user_type, t.user_id) == expected }));
        assert!(!delta.writes.is_empty());
        evt.issuer = Some("https://another.example/iam".into());
        let other = translator()
            .translate(&to_json(HiveDomainEvent::MemberJoined(evt.clone())))
            .unwrap()
            .unwrap();
        assert_ne!(delta.writes[0].user_id, other.writes[0].user_id);
        evt.issuer = None;
        assert_eq!(
            translator()
                .translate(&to_json(HiveDomainEvent::MemberJoined(evt)))
                .unwrap_err()
                .to_string(),
            "EVENT_PRINCIPAL_UNRESOLVED"
        );
    }

    #[test]
    fn organization_deletion_keeps_same_sub_authorities_distinct_and_rejects_legacy() {
        let org = Uuid::new_v4();
        let same_sub = Uuid::new_v4();
        let mut event = OrganizationDeletedEvent::new(
            org,
            "ACME".into(),
            same_sub,
            vec![same_sub],
            same_sub,
            Utc::now(),
        );
        assert_eq!(
            translator()
                .translate(&to_json(HiveDomainEvent::OrganizationDeleted(
                    event.clone()
                )))
                .unwrap_err()
                .to_string(),
            "EVENT_PRINCIPAL_UNRESOLVED"
        );
        event.owner_issuer = Some(PLATFORM.into());
        event.member_principals = Some(vec![
            hive_events::MemberPrincipal {
                issuer: "https://org1.example/iam".into(),
                user_id: same_sub,
            },
            hive_events::MemberPrincipal {
                issuer: "https://org2.example/iam".into(),
                user_id: same_sub,
            },
        ]);
        let delta = translator()
            .translate(&to_json(HiveDomainEvent::OrganizationDeleted(
                event.clone(),
            )))
            .unwrap()
            .unwrap();
        let users: std::collections::HashSet<_> =
            delta.deletes.iter().map(|tuple| &tuple.user_id).collect();
        assert_eq!(users.len(), 3);
        assert_eq!(delta.deletes.len(), 12);
        assert!(delta.writes.is_empty());
        event
            .member_principals
            .as_mut()
            .unwrap()
            .push(hive_events::MemberPrincipal {
                issuer: String::new(),
                user_id: Uuid::new_v4(),
            });
        assert_eq!(
            translator()
                .translate(&to_json(HiveDomainEvent::OrganizationDeleted(event)))
                .unwrap_err()
                .to_string(),
            "EVENT_PRINCIPAL_UNRESOLVED"
        );
    }
}
