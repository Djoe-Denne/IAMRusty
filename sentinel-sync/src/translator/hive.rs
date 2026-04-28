//! Hive event -> `OpenFGA` tuple translation.
//!
//! Maps organization lifecycle and membership events from `hive-events`
//! onto tuples for the `organization` `OpenFGA` type.
//!
//! `OrganizationDeleted` intentionally emits no per-tuple deletes here: the
//! store-level clean-up of every dangling `organization:{id}` tuple is best
//! done by a periodic garbage-collect job because it requires an
//! `ListObjects`/`ReadTuples` sweep that the event payload cannot enumerate
//! on its own. A TODO tracks that job.

use anyhow::Result;
use hive_events::{HiveDomainEvent, Role};

use super::{Translator, TupleDelta};
use crate::fga_client::Tuple;

#[derive(Default)]
pub struct HiveTranslator;

impl HiveTranslator {
    pub const fn new() -> Self {
        Self
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
    user_id: uuid::Uuid,
    roles: &[Role],
) -> Vec<Tuple> {
    let mut out = Vec::new();
    for role in roles {
        if let Some(relation) = role_to_org_relation(&role.permission) {
            out.push(Tuple::user(
                "organization",
                organization_id,
                relation,
                user_id,
            ));
        }
    }
    // Every member also gets the bare `member` relation so checks for
    // "is this user in the org?" work even without any explicit role.
    out.push(Tuple::user(
        "organization",
        organization_id,
        "member",
        user_id,
    ));
    out
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
            HiveDomainEvent::OrganizationCreated(evt) => TupleDelta::default().write(Tuple::user(
                "organization",
                evt.organization_id,
                "owner",
                evt.owner_user_id,
            )),

            HiveDomainEvent::MemberJoined(evt) => {
                let mut d = TupleDelta::default();
                for t in role_tuples_for_member(evt.organization_id, evt.user_id, &evt.roles) {
                    d = d.write(t);
                }
                d
            }

            HiveDomainEvent::MemberRemoved(evt) => {
                // Remove both the base membership and any role tuples. We
                // cannot know the exact role set at removal time, so we
                // delete every known organization relation for this user.
                let mut d = TupleDelta::default();
                for relation in ["owner", "admin", "member", "viewer"] {
                    d = d.delete(Tuple::user(
                        "organization",
                        evt.organization_id,
                        relation,
                        evt.user_id,
                    ));
                }
                d
            }

            // Lifecycle and sync events that do not move any authorization
            // fact in OpenFGA.
            HiveDomainEvent::OrganizationUpdated(_)
            | HiveDomainEvent::OrganizationDeleted(_)
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
    use hive_events::{MemberJoinedEvent, OrganizationCreatedEvent, Role};
    use uuid::Uuid;

    fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
        // Wrap into the HiveDomainEvent tagged form by round-tripping.
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn organization_created_writes_owner_tuple() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = HiveDomainEvent::OrganizationCreated(OrganizationCreatedEvent::new(
            org_id,
            "ACME".to_string(),
            "acme".to_string(),
            user_id,
            Utc::now(),
        ));
        let translator = HiveTranslator::new();
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
        let evt = HiveDomainEvent::MemberJoined(MemberJoinedEvent::new(
            org_id,
            "ACME".to_string(),
            user_id,
            vec![Role::new("admin".to_string(), "organization".to_string())],
            Utc::now(),
        ));
        let translator = HiveTranslator::new();
        let delta = translator.translate(&to_json(evt)).unwrap().unwrap();
        let relations: Vec<_> = delta.writes.iter().map(|t| t.relation.as_str()).collect();
        assert!(relations.contains(&"admin"));
        assert!(relations.contains(&"member"));
    }
}
