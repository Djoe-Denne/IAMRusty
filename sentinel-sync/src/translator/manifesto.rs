//! Manifesto event -> `OpenFGA` tuple translation.
//!
//! Projects live under an optional parent organization (`owner_type ==
//! "organization"`). Components live under their project. Members and
//! explicit permission grants attach users to a project's `member`,
//! `admin`, or `viewer` relation.

use anyhow::Result;
use manifesto_events::ManifestoDomainEvent;

use super::{Translator, TupleDelta};
use crate::fga_client::Tuple;

#[derive(Default)]
pub struct ManifestoTranslator;

impl ManifestoTranslator {
    pub const fn new() -> Self {
        Self
    }
}

/// Translate the string `permission` field on Manifesto events into the
/// corresponding `OpenFGA` relation on `project` or `component`.
///
/// Unknown permissions yield `None`; the translator skips the tuple rather
/// than fabricating a relation.
fn permission_to_relation(permission: &str) -> Option<&'static str> {
    match permission.to_lowercase().as_str() {
        "owner" => Some("owner"),
        "admin" => Some("admin"),
        "write" => Some("member"),
        "read" => Some("viewer"),
        _ => None,
    }
}

/// Map the string `resource` on Manifesto events to an `OpenFGA` object type.
/// Anything unrecognized falls back to `project`, which preserves the old
/// Casbin "unidentified resource" semantics.
fn resource_to_object_type(resource: &str) -> &'static str {
    match resource.to_lowercase().as_str() {
        "component" => "component",
        _ => "project",
    }
}

impl Translator for ManifestoTranslator {
    fn name(&self) -> &'static str {
        "manifesto"
    }

    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>> {
        let event: ManifestoDomainEvent = match serde_json::from_value(raw_event.clone()) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        let delta = match event {
            ManifestoDomainEvent::ProjectCreated(evt) => {
                let mut d = TupleDelta::default().write(Tuple::user(
                    "project",
                    evt.project_id,
                    "owner",
                    evt.created_by,
                ));
                if evt.owner_type == "organization" {
                    d = d.write(Tuple::object(
                        "project",
                        evt.project_id,
                        "organization",
                        "organization",
                        evt.owner_id,
                    ));
                }
                d
            }

            ManifestoDomainEvent::ComponentAdded(evt) => {
                TupleDelta::default().write(Tuple::object(
                    "component",
                    evt.component_id,
                    "project",
                    "project",
                    evt.project_id,
                ))
            }

            ManifestoDomainEvent::ComponentRemoved(evt) => {
                TupleDelta::default().delete(Tuple::object(
                    "component",
                    evt.component_id,
                    "project",
                    "project",
                    evt.project_id,
                ))
            }

            ManifestoDomainEvent::MemberAdded(evt) => {
                // Every member gets the base `project#member` tuple. The
                // initial permission/resource pair, when it resolves, adds a
                // more privileged tuple on top.
                let mut d = TupleDelta::default().write(Tuple::user(
                    "project",
                    evt.project_id,
                    "member",
                    evt.user_id,
                ));
                if let Some(relation) = permission_to_relation(&evt.initial_permission) {
                    let object_type = resource_to_object_type(&evt.initial_resource);
                    d = d.write(Tuple::user(
                        object_type,
                        // Initial grants are always project-scoped; component-
                        // scoped grants use explicit PermissionGranted events.
                        evt.project_id,
                        relation,
                        evt.user_id,
                    ));
                }
                d
            }

            ManifestoDomainEvent::MemberRemoved(evt) => {
                let mut d = TupleDelta::default();
                for relation in ["owner", "admin", "member", "viewer"] {
                    d = d.delete(Tuple::user(
                        "project",
                        evt.project_id,
                        relation,
                        evt.user_id,
                    ));
                }
                d
            }

            ManifestoDomainEvent::PermissionGranted(evt) => {
                match permission_to_relation(&evt.permission) {
                    Some(relation) => {
                        let object_type = resource_to_object_type(&evt.resource);
                        TupleDelta::default().write(Tuple::user(
                            object_type,
                            evt.project_id,
                            relation,
                            evt.user_id,
                        ))
                    }
                    None => TupleDelta::default(),
                }
            }

            ManifestoDomainEvent::PermissionRevoked(evt) => {
                // PermissionRevoked carries the `resource` string but not the
                // original permission verb. Delete every relation on that
                // object type for the user; OpenFGA is idempotent on missing
                // tuples.
                let object_type = resource_to_object_type(&evt.resource);
                let mut d = TupleDelta::default();
                for relation in ["owner", "admin", "member", "viewer"] {
                    d = d.delete(Tuple::user(
                        object_type,
                        evt.project_id,
                        relation,
                        evt.user_id,
                    ));
                }
                d
            }

            // Lifecycle events that do not move authorization state. Project
            // deletion would ideally sweep every tuple on `project:{id}`; the
            // cleanup is tracked in references/sentinel-sync-worker.md as a
            // garbage-collect job rather than a single-event delete.
            ManifestoDomainEvent::ProjectUpdated(_)
            | ManifestoDomainEvent::ProjectDeleted(_)
            | ManifestoDomainEvent::ProjectPublished(_)
            | ManifestoDomainEvent::ProjectArchived(_)
            | ManifestoDomainEvent::ComponentStatusChanged(_)
            | ManifestoDomainEvent::MemberPermissionsUpdated(_) => TupleDelta::default(),
        };

        Ok(Some(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use manifesto_events::{
        ComponentAddedEvent, MemberAddedEvent, PermissionGrantedEvent, ProjectCreatedEvent,
    };
    use uuid::Uuid;

    fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn project_created_under_org_writes_owner_and_parent_tuple() {
        let project_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let creator = Uuid::new_v4();
        let evt = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
            project_id,
            "demo".into(),
            "organization".into(),
            org_id,
            creator,
            "public".into(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.writes.len(), 2);
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "organization" && t.user_id == org_id.to_string()));
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "owner" && t.user_id == creator.to_string()));
    }

    #[test]
    fn component_added_links_component_to_project() {
        let project_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::ComponentAdded(ComponentAddedEvent::new(
            project_id,
            component_id,
            "panel".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].object_type, "component");
        assert_eq!(delta.writes[0].relation, "project");
        assert_eq!(delta.writes[0].user_type, "project");
    }

    #[test]
    fn member_added_writes_member_plus_initial_role() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::MemberAdded(MemberAddedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            "admin".into(),
            "project".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        let relations: Vec<_> = delta.writes.iter().map(|t| t.relation.as_str()).collect();
        assert!(relations.contains(&"admin"));
        assert!(relations.contains(&"member"));
    }

    #[test]
    fn permission_granted_on_component_uses_component_type() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::PermissionGranted(PermissionGrantedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            "component".into(),
            "write".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].object_type, "component");
        assert_eq!(delta.writes[0].relation, "member");
    }
}
