//! Manifesto event -> `OpenFGA` tuple translation.
//!
//! v2 payloads carry exact `object_type` / `object_id` / `permission`. Incomplete
//! v1 revoke/replace events produce an empty delta (stale allow) rather than a
//! relation wipe. Deploy this consumer before Manifesto starts emitting v2.

use anyhow::Result;
use manifesto_events::{
    exact_user_tuple, fga_relation, AuthzTuple, ManifestoDomainEvent,
    MemberPermissionsUpdatedEvent, PermissionGrantedEvent, PermissionRevokedEvent,
    ResourcePermission,
};
use uuid::Uuid;

use super::{Translator, TupleDelta};
use crate::fga_client::Tuple;

#[derive(Default)]
pub struct ManifestoTranslator;

impl ManifestoTranslator {
    pub const fn new() -> Self {
        Self
    }
}

impl Translator for ManifestoTranslator {
    fn name(&self) -> &'static str {
        "manifesto"
    }

    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>> {
        let Ok(event) = serde_json::from_value(raw_event.clone()) else {
            return Ok(None);
        };
        Ok(Some(translate_event(&event)))
    }
}

const fn is_public_visibility(visibility: &str) -> bool {
    visibility.eq_ignore_ascii_case("public")
}

const fn is_internal_visibility(visibility: &str) -> bool {
    visibility.eq_ignore_ascii_case("internal")
}

fn org_member_viewer_userset(project_id: Uuid, owner_type: &str, owner_id: Uuid) -> Option<Tuple> {
    if owner_type == "organization" {
        Some(Tuple::userset(
            "project",
            project_id,
            "viewer",
            "organization",
            owner_id,
            "member",
        ))
    } else {
        None
    }
}

fn tuple_from_authz(t: &AuthzTuple) -> Tuple {
    Tuple::user(&t.object_type, t.object_id, &t.relation, t.user_id)
}

fn grant_tuple(
    resource: &str,
    permission: &str,
    object_type: Option<&str>,
    object_id: Option<Uuid>,
    project_id: Uuid,
    user_id: Uuid,
) -> Option<Tuple> {
    if let (Some(object_type), Some(object_id)) = (object_type, object_id) {
        return fga_relation(object_type, permission)
            .map(|relation| Tuple::user(object_type, object_id, relation, user_id));
    }
    exact_user_tuple(resource, permission, project_id, user_id).map(|t| tuple_from_authz(&t))
}

fn resource_permission_tuple(
    perm: &ResourcePermission,
    project_id: Uuid,
    user_id: Uuid,
) -> Option<Tuple> {
    perm.to_tuple(project_id, user_id)
        .map(|t| tuple_from_authz(&t))
}

fn project_created_delta(evt: &manifesto_events::ProjectCreatedEvent) -> TupleDelta {
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
    if is_public_visibility(&evt.visibility) {
        d = d.write(Tuple::wildcard_user("project", evt.project_id, "viewer"));
    }
    if is_internal_visibility(&evt.visibility) {
        if let Some(userset) =
            org_member_viewer_userset(evt.project_id, &evt.owner_type, evt.owner_id)
        {
            d = d.write(userset);
        }
    }
    d
}

fn project_deleted_delta(evt: &manifesto_events::ProjectDeletedEvent) -> TupleDelta {
    let v1_incomplete = evt.member_user_ids.is_empty()
        && evt.component_ids.is_empty()
        && evt.owner_type.is_none()
        && evt.owner_id.is_none();
    if v1_incomplete {
        return TupleDelta::default();
    }
    let mut d =
        TupleDelta::default().delete(Tuple::wildcard_user("project", evt.project_id, "viewer"));
    if let (Some(owner_type), Some(owner_id)) = (&evt.owner_type, evt.owner_id) {
        if let Some(userset) = org_member_viewer_userset(evt.project_id, owner_type, owner_id) {
            d = d.delete(userset);
        }
    }
    for user_id in &evt.member_user_ids {
        for relation in ["owner", "admin", "member", "viewer"] {
            d = d.delete(Tuple::user("project", evt.project_id, relation, *user_id));
        }
        for component_id in &evt.component_ids {
            d = d.delete(Tuple::user("component", *component_id, "viewer", *user_id));
            d = d.delete(Tuple::user("component", *component_id, "editor", *user_id));
        }
    }
    for component_id in &evt.component_ids {
        d = d.delete(Tuple::object(
            "component",
            *component_id,
            "project",
            "project",
            evt.project_id,
        ));
    }
    d
}

fn member_permissions_updated_delta(evt: &MemberPermissionsUpdatedEvent) -> TupleDelta {
    if evt.previous.is_empty() {
        return TupleDelta::default();
    }
    let mut d = TupleDelta::default();
    for perm in &evt.previous {
        if let Some(tuple) = resource_permission_tuple(perm, evt.project_id, evt.user_id) {
            d = d.delete(tuple);
        }
    }
    for perm in &evt.permissions {
        if let Some(tuple) = resource_permission_tuple(perm, evt.project_id, evt.user_id) {
            d = d.write(tuple);
        }
    }
    d
}

fn component_added_delta(evt: &manifesto_events::ComponentAddedEvent) -> TupleDelta {
    TupleDelta::default().write(Tuple::object(
        "component",
        evt.component_id,
        "project",
        "project",
        evt.project_id,
    ))
}

fn component_removed_delta(evt: &manifesto_events::ComponentRemovedEvent) -> TupleDelta {
    TupleDelta::default().delete(Tuple::object(
        "component",
        evt.component_id,
        "project",
        "project",
        evt.project_id,
    ))
}

fn member_added_delta(evt: &manifesto_events::MemberAddedEvent) -> TupleDelta {
    grant_tuple(
        &evt.initial_resource,
        &evt.initial_permission,
        evt.object_type.as_deref(),
        evt.object_id,
        evt.project_id,
        evt.user_id,
    )
    .map_or_else(TupleDelta::default, |tuple| {
        TupleDelta::default().write(tuple)
    })
}

fn member_removed_delta(evt: &manifesto_events::MemberRemovedEvent) -> TupleDelta {
    if evt.tuples.is_empty() {
        return TupleDelta::default();
    }
    let mut d = TupleDelta::default();
    for tuple in &evt.tuples {
        d = d.delete(tuple_from_authz(tuple));
    }
    d
}

fn permission_granted_delta(evt: &PermissionGrantedEvent) -> TupleDelta {
    grant_tuple(
        &evt.resource,
        &evt.permission,
        evt.object_type.as_deref(),
        evt.object_id,
        evt.project_id,
        evt.user_id,
    )
    .map_or_else(TupleDelta::default, |tuple| {
        TupleDelta::default().write(tuple)
    })
}

fn permission_revoked_delta(evt: &PermissionRevokedEvent) -> TupleDelta {
    let Some(permission) = evt.permission.as_deref() else {
        return TupleDelta::default();
    };
    grant_tuple(
        &evt.resource,
        permission,
        evt.object_type.as_deref(),
        evt.object_id,
        evt.project_id,
        evt.user_id,
    )
    .map_or_else(TupleDelta::default, |tuple| {
        TupleDelta::default().delete(tuple)
    })
}

fn project_visibility_changed_delta(
    evt: &manifesto_events::ProjectVisibilityChangedEvent,
) -> TupleDelta {
    let old_public = is_public_visibility(&evt.old_visibility);
    let new_public = is_public_visibility(&evt.new_visibility);
    let old_internal = is_internal_visibility(&evt.old_visibility);
    let new_internal = is_internal_visibility(&evt.new_visibility);
    let wildcard = Tuple::wildcard_user("project", evt.project_id, "viewer");
    let mut d = TupleDelta::default();
    if !old_public && new_public {
        d = d.write(wildcard);
    } else if old_public && !new_public {
        d = d.delete(wildcard);
    }
    if let Some(userset) = org_member_viewer_userset(evt.project_id, &evt.owner_type, evt.owner_id)
    {
        if !old_internal && new_internal {
            d = d.write(userset);
        } else if old_internal && !new_internal {
            d = d.delete(userset);
        }
    }
    d
}

fn project_suspended_delta(evt: &manifesto_events::ProjectSuspendedEvent) -> TupleDelta {
    let mut d =
        TupleDelta::default().delete(Tuple::wildcard_user("project", evt.project_id, "viewer"));
    if let Some(userset) = org_member_viewer_userset(evt.project_id, &evt.owner_type, evt.owner_id)
    {
        d = d.delete(userset);
    }
    d
}

fn project_resumed_delta(evt: &manifesto_events::ProjectResumedEvent) -> TupleDelta {
    let mut d = TupleDelta::default();
    if is_public_visibility(&evt.visibility) {
        d = d.write(Tuple::wildcard_user("project", evt.project_id, "viewer"));
    }
    if is_internal_visibility(&evt.visibility) {
        if let Some(userset) =
            org_member_viewer_userset(evt.project_id, &evt.owner_type, evt.owner_id)
        {
            d = d.write(userset);
        }
    }
    d
}

fn ownership_transferred_delta(
    evt: &manifesto_events::ProjectOwnershipTransferredEvent,
) -> TupleDelta {
    let mut d = TupleDelta::default()
        .delete(Tuple::user(
            "project",
            evt.project_id,
            "owner",
            evt.from_user_id,
        ))
        .write(Tuple::user(
            "project",
            evt.project_id,
            "admin",
            evt.from_user_id,
        ))
        .write(Tuple::user(
            "project",
            evt.project_id,
            "owner",
            evt.to_user_id,
        ));
    if evt.owner_type.as_deref() == Some("personal") {
        if let Some(previous_owner_id) = evt.previous_owner_id {
            d = d.delete(Tuple::object(
                "project",
                evt.project_id,
                "organization",
                "user",
                previous_owner_id,
            ));
        }
        // Personal projects have no organization parent tuple; owner_id is the user.
        let _ = evt.new_owner_id;
    }
    d
}

fn translate_event(event: &ManifestoDomainEvent) -> TupleDelta {
    let delta = match event {
        ManifestoDomainEvent::ProjectCreated(evt) => project_created_delta(evt),
        ManifestoDomainEvent::ComponentAdded(evt) => component_added_delta(evt),
        ManifestoDomainEvent::ComponentRemoved(evt) => component_removed_delta(evt),
        ManifestoDomainEvent::MemberAdded(evt) => member_added_delta(evt),
        ManifestoDomainEvent::MemberRemoved(evt) => member_removed_delta(evt),
        ManifestoDomainEvent::PermissionGranted(evt) => permission_granted_delta(evt),
        ManifestoDomainEvent::PermissionRevoked(evt) => permission_revoked_delta(evt),
        ManifestoDomainEvent::ProjectDeleted(evt) => project_deleted_delta(evt),
        ManifestoDomainEvent::MemberPermissionsUpdated(evt) => {
            member_permissions_updated_delta(evt)
        }
        ManifestoDomainEvent::ProjectVisibilityChanged(evt) => {
            project_visibility_changed_delta(evt)
        }
        ManifestoDomainEvent::ProjectArchived(evt) => {
            let mut d = TupleDelta::default().delete(Tuple::wildcard_user(
                "project",
                evt.project_id,
                "viewer",
            ));
            if let Some(userset) =
                org_member_viewer_userset(evt.project_id, &evt.owner_type, evt.owner_id)
            {
                d = d.delete(userset);
            }
            d
        }
        ManifestoDomainEvent::ProjectSuspended(evt) => project_suspended_delta(evt),
        ManifestoDomainEvent::ProjectResumed(evt) => project_resumed_delta(evt),
        ManifestoDomainEvent::ProjectOwnershipTransferred(evt) => ownership_transferred_delta(evt),
        ManifestoDomainEvent::ProjectPublished(_)
        | ManifestoDomainEvent::ProjectUpdated(_)
        | ManifestoDomainEvent::ComponentStatusChanged(_) => TupleDelta::default(),
    };
    delta.normalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use manifesto_events::{
        AuthzTuple, ComponentAddedEvent, MemberAddedEvent, MemberPermissionsUpdatedEvent,
        MemberRemovedEvent, PermissionGrantedEvent, PermissionRevokedEvent, ProjectArchivedEvent,
        ProjectCreatedEvent, ProjectDeletedEvent, ProjectOwnershipTransferredEvent,
        ProjectPublishedEvent, ProjectResumedEvent, ProjectSuspendedEvent,
        ProjectVisibilityChangedEvent, ResourcePermission,
    };

    fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
        serde_json::to_value(value).expect("Serialize")
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
        assert_eq!(delta.writes.len(), 3);
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "organization" && t.user_id == org_id.to_string()));
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "owner" && t.user_id == creator.to_string()));
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "viewer" && t.user_id == "*"));
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
    fn member_added_writes_only_initial_role() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::MemberAdded(MemberAddedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            "read".into(),
            "project".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].relation, "viewer");
        assert_eq!(delta.writes[0].object_id, project_id.to_string());
    }

    #[test]
    fn permission_granted_on_component_instance_uses_editor_and_uuid() {
        let project_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::PermissionGranted(PermissionGrantedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            component_id.to_string(),
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
        assert_eq!(delta.writes[0].object_id, component_id.to_string());
        assert_eq!(delta.writes[0].relation, "editor");
    }

    #[test]
    fn permission_revoked_v1_without_permission_is_noop() {
        let raw = serde_json::json!({
            "event_type": "permission_revoked",
            "data": {
                "event_id": Uuid::new_v4(),
                "aggregate_id": Uuid::new_v4(),
                "event_type": "permission_revoked",
                "occurred_at": Utc::now(),
                "version": 1,
                "metadata": {},
                "project_id": Uuid::new_v4(),
                "member_id": Uuid::new_v4(),
                "user_id": Uuid::new_v4(),
                "resource": "project",
                "revoked_by": Uuid::new_v4(),
                "revoked_at": Utc::now()
            }
        });
        let delta = ManifestoTranslator::new().translate(&raw).unwrap().unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn permission_revoked_v2_deletes_one_tuple() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::PermissionRevoked(PermissionRevokedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            "project".into(),
            "admin".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.deletes.len(), 1);
        assert_eq!(delta.deletes[0].relation, "admin");
        assert_eq!(delta.deletes[0].object_id, project_id.to_string());
        assert!(delta.writes.is_empty());
    }

    #[test]
    fn project_deleted_v1_without_authz_lists_is_noop() {
        let evt = ManifestoDomainEvent::ProjectDeleted(ProjectDeletedEvent::new(
            Uuid::new_v4(),
            "gone".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn project_deleted_v2_drops_wildcard_and_member_tuples() {
        let project_id = Uuid::new_v4();
        let actor = Uuid::new_v4();
        let evt = ManifestoDomainEvent::ProjectDeleted(ProjectDeletedEvent::with_authz(
            project_id,
            "gone".into(),
            actor,
            Utc::now(),
            vec![actor],
            Vec::new(),
            Some("personal".into()),
            Some(actor),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert!(delta.writes.is_empty());
        assert!(delta
            .deletes
            .iter()
            .any(|t| t.user_id == "*" && t.relation == "viewer"));
        assert!(delta
            .deletes
            .iter()
            .any(|t| t.user_id == actor.to_string() && t.relation == "owner"));
    }

    #[test]
    fn member_permissions_updated_without_previous_is_noop() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt =
            ManifestoDomainEvent::MemberPermissionsUpdated(MemberPermissionsUpdatedEvent::new(
                project_id,
                Uuid::new_v4(),
                user_id,
                vec![ResourcePermission::new(
                    "project".into(),
                    "write".into(),
                    project_id,
                )],
                Uuid::new_v4(),
                Utc::now(),
            ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn member_permissions_updated_with_previous_deletes_exact_prior() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::MemberPermissionsUpdated(
            MemberPermissionsUpdatedEvent::with_previous(
                project_id,
                Uuid::new_v4(),
                user_id,
                vec![ResourcePermission::new(
                    "project".into(),
                    "admin".into(),
                    project_id,
                )],
                vec![ResourcePermission::new(
                    "project".into(),
                    "write".into(),
                    project_id,
                )],
                Uuid::new_v4(),
                Utc::now(),
            ),
        );
        let delta = translate_event(&evt);
        assert!(delta.deletes.iter().any(|t| t.relation == "admin"));
        assert!(delta.writes.iter().any(|t| t.relation == "member"));
    }

    #[test]
    fn member_removed_v1_without_tuples_is_noop() {
        let evt = ManifestoDomainEvent::MemberRemoved(MemberRemovedEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = translate_event(&evt);
        assert!(delta.is_empty());
    }

    #[test]
    fn member_removed_v2_deletes_exact_tuples_only() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::MemberRemoved(MemberRemovedEvent::with_tuples(
            project_id,
            Uuid::new_v4(),
            user_id,
            Uuid::new_v4(),
            Utc::now(),
            vec![
                AuthzTuple::new("project", project_id, "viewer", user_id),
                AuthzTuple::new("component", component_id, "viewer", user_id),
            ],
            vec![component_id],
        ));
        let delta = translate_event(&evt);
        assert_eq!(delta.deletes.len(), 2);
        assert!(delta
            .deletes
            .iter()
            .any(|t| t.object_type == "component" && t.object_id == component_id.to_string()));
        assert!(!delta.deletes.iter().any(|t| t.relation == "owner"));
    }

    #[test]
    fn ownership_transfer_moves_owner_and_keeps_former_as_admin() {
        let project_id = Uuid::new_v4();
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let delta = translate_event(&ManifestoDomainEvent::ProjectOwnershipTransferred(
            ProjectOwnershipTransferredEvent::new(
                project_id,
                from,
                to,
                from,
                Utc::now(),
                Some("personal".into()),
                Some(from),
                Some(to),
            ),
        ));
        assert!(delta
            .deletes
            .iter()
            .any(|t| t.relation == "owner" && t.user_id == from.to_string()));
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "owner" && t.user_id == to.to_string()));
        assert!(delta
            .writes
            .iter()
            .any(|t| t.relation == "admin" && t.user_id == from.to_string()));
    }

    #[test]
    fn suspend_deletes_wildcard_and_resume_public_rewrites_it() {
        let project_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let suspended = translate_event(&ManifestoDomainEvent::ProjectSuspended(
            ProjectSuspendedEvent::new(
                project_id,
                "demo".into(),
                "organization".into(),
                org_id,
                "public".into(),
                Uuid::new_v4(),
                Utc::now(),
            ),
        ));
        assert!(suspended.deletes.iter().any(|t| t.user_id == "*"));
        let resumed = translate_event(&ManifestoDomainEvent::ProjectResumed(
            ProjectResumedEvent::new(
                project_id,
                "demo".into(),
                "organization".into(),
                org_id,
                "public".into(),
                Uuid::new_v4(),
                Utc::now(),
            ),
        ));
        assert!(resumed.writes.iter().any(|t| t.user_id == "*"));
    }

    fn visibility_event(
        old: &str,
        new: &str,
        owner_type: &str,
        owner_id: Uuid,
    ) -> ManifestoDomainEvent {
        ManifestoDomainEvent::ProjectVisibilityChanged(ProjectVisibilityChangedEvent::new(
            Uuid::new_v4(),
            owner_type.into(),
            owner_id,
            old.into(),
            new.into(),
            Uuid::new_v4(),
            Utc::now(),
        ))
    }

    #[test]
    fn project_published_does_not_write_wildcard() {
        let delta = translate_event(&ManifestoDomainEvent::ProjectPublished(
            ProjectPublishedEvent::new(Uuid::new_v4(), "demo".into(), Uuid::new_v4(), Utc::now()),
        ));
        assert!(delta.is_empty());
    }

    #[test]
    fn project_archived_deletes_wildcard_and_org_userset() {
        let project_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let delta = translate_event(&ManifestoDomainEvent::ProjectArchived(
            ProjectArchivedEvent::new(
                project_id,
                "demo".into(),
                "organization".into(),
                org_id,
                Uuid::new_v4(),
                Utc::now(),
            ),
        ));
        assert!(delta.writes.is_empty());
        assert_eq!(delta.deletes.len(), 2);
        assert!(delta.deletes.iter().any(|t| t.user_id == "*"));
        assert!(delta
            .deletes
            .iter()
            .any(|t| t.user_id == format!("{org_id}#member")));
    }

    #[test]
    fn project_created_internal_org_writes_member_userset_not_wildcard() {
        let project_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
            project_id,
            "demo".into(),
            "organization".into(),
            org_id,
            Uuid::new_v4(),
            "internal".into(),
            Utc::now(),
        ));
        let delta = ManifestoTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert!(delta
            .writes
            .iter()
            .any(|t| t.user_id == format!("{org_id}#member")));
        assert!(!delta.writes.iter().any(|t| t.user_id == "*"));
    }

    #[test]
    fn project_visibility_changed_syncs_wildcard_only_for_public() {
        let owner_id = Uuid::new_v4();
        let cases = [
            ("private", "public", 1, 0),
            ("public", "private", 0, 1),
            ("public", "internal", 0, 1),
            ("private", "internal", 0, 0),
            ("internal", "private", 0, 0),
            ("public", "public", 0, 0),
        ];
        for (old, new, writes, deletes) in cases {
            let delta = translate_event(&visibility_event(old, new, "personal", owner_id));
            assert_eq!(
                (delta.writes.len(), delta.deletes.len()),
                (writes, deletes),
                "personal {old} -> {new}"
            );
        }
    }

    #[test]
    fn project_visibility_changed_syncs_org_internal_userset() {
        let org_id = Uuid::new_v4();
        let private_to_internal = translate_event(&visibility_event(
            "private",
            "internal",
            "organization",
            org_id,
        ));
        assert_eq!(private_to_internal.writes.len(), 1);
        assert_eq!(
            private_to_internal.writes[0].user_id,
            format!("{org_id}#member")
        );
        assert!(private_to_internal.deletes.is_empty());

        let public_to_internal = translate_event(&visibility_event(
            "public",
            "internal",
            "organization",
            org_id,
        ));
        assert_eq!(public_to_internal.writes.len(), 1);
        assert_eq!(public_to_internal.deletes.len(), 1);
        assert_eq!(public_to_internal.deletes[0].user_id, "*");

        let internal_to_public = translate_event(&visibility_event(
            "internal",
            "public",
            "organization",
            org_id,
        ));
        assert_eq!(internal_to_public.writes.len(), 1);
        assert_eq!(internal_to_public.writes[0].user_id, "*");
        assert_eq!(internal_to_public.deletes.len(), 1);
        assert_eq!(
            internal_to_public.deletes[0].user_id,
            format!("{org_id}#member")
        );
    }

    #[test]
    fn member_added_generic_component_writes_project_component_viewer() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = ManifestoDomainEvent::MemberAdded(MemberAddedEvent::new(
            project_id,
            Uuid::new_v4(),
            user_id,
            "read".into(),
            "component".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let delta = translate_event(&evt);
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].object_type, "project");
        assert_eq!(delta.writes[0].object_id, project_id.to_string());
        assert_eq!(delta.writes[0].relation, "component_viewer");
    }

    #[test]
    fn grant_tuple_prefers_explicit_object_over_resource_name() {
        let project_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let tuple = grant_tuple(
            "taskboard",
            "write",
            Some("component"),
            Some(component_id),
            project_id,
            user_id,
        )
        .expect("tuple");
        assert_eq!(tuple.object_type, "component");
        assert_eq!(tuple.object_id, component_id.to_string());
        assert_eq!(tuple.relation, "editor");
    }
}
