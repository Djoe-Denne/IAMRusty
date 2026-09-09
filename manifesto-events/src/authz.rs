//! Shared AuthZ resource parsing for Manifesto events and sentinel-sync.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One exact `OpenFGA` tuple carried on a domain event so the translator
/// never has to invent object ids or wipe unrelated relations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthzTuple {
    pub object_type: String,
    pub object_id: Uuid,
    pub relation: String,
    pub user_id: Uuid,
}

impl AuthzTuple {
    #[must_use]
    pub fn new(
        object_type: impl Into<String>,
        object_id: Uuid,
        relation: impl Into<String>,
        user_id: Uuid,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            object_id,
            relation: relation.into(),
            user_id,
        }
    }
}

/// Resolved `OpenFGA` object for a Manifesto resource string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthzTarget {
    pub object_type: String,
    pub object_id: Uuid,
}

/// Map a Manifesto resource identifier to an `OpenFGA` object.
///
/// * `"project"` / `"member"` → `project:{project_id}`
/// * a UUID, optionally prefixed with `component:` → `component:{id}`
/// * generic `"component"` (type-level) → `None` (use [`exact_user_tuple`])
#[must_use]
pub fn resolve_authz_target(resource: &str, project_id: Uuid) -> Option<AuthzTarget> {
    let trimmed = resource.trim();
    if trimmed.eq_ignore_ascii_case("project") || trimmed.eq_ignore_ascii_case("member") {
        return Some(AuthzTarget {
            object_type: "project".to_string(),
            object_id: project_id,
        });
    }
    if trimmed.eq_ignore_ascii_case("component") {
        return None;
    }
    if let Ok(id) = Uuid::parse_str(trimmed) {
        return Some(AuthzTarget {
            object_type: "component".to_string(),
            object_id: id,
        });
    }
    if let Some((kind, id_str)) = trimmed.split_once(':') {
        if let Ok(id) = Uuid::parse_str(id_str) {
            if kind.eq_ignore_ascii_case("component") {
                return Some(AuthzTarget {
                    object_type: "component".to_string(),
                    object_id: id,
                });
            }
            if kind.eq_ignore_ascii_case("project") {
                return Some(AuthzTarget {
                    object_type: "project".to_string(),
                    object_id: id,
                });
            }
        }
    }
    None
}

/// Map a Manifesto permission verb onto the `OpenFGA` relation for `object_type`.
///
/// Component objects only accept `viewer` / `editor` in `openfga/model.fga`.
#[must_use]
pub fn fga_relation(object_type: &str, permission: &str) -> Option<&'static str> {
    match (
        object_type.to_ascii_lowercase().as_str(),
        permission.to_ascii_lowercase().as_str(),
    ) {
        ("project", "owner") => Some("owner"),
        ("project", "admin") => Some("admin"),
        ("project", "write") => Some("member"),
        ("project", "read") => Some("viewer"),
        ("project", "component_viewer") => Some("component_viewer"),
        ("project", "component_editor") => Some("component_editor"),
        ("component", "write" | "admin") => Some("editor"),
        ("component", "read") => Some("viewer"),
        _ => None,
    }
}

fn generic_component_relation(permission: &str) -> Option<&'static str> {
    match permission.to_ascii_lowercase().as_str() {
        "read" => Some("component_viewer"),
        "write" | "admin" | "owner" => Some("component_editor"),
        _ => None,
    }
}

/// Build the exact tuple for a grant/revoke, or `None` when the resource is
/// unrecognized (caller must not invent a wipe).
///
/// Generic `"component"` maps to `project:{id}#component_viewer|component_editor`.
#[must_use]
pub fn exact_user_tuple(
    resource: &str,
    permission: &str,
    project_id: Uuid,
    user_id: Uuid,
) -> Option<AuthzTuple> {
    let trimmed = resource.trim();
    if trimmed.eq_ignore_ascii_case("component") {
        let relation = generic_component_relation(permission)?;
        return Some(AuthzTuple::new("project", project_id, relation, user_id));
    }
    let target = resolve_authz_target(resource, project_id)?;
    let relation = fga_relation(&target.object_type, permission)?;
    Some(AuthzTuple::new(
        target.object_type,
        target.object_id,
        relation,
        user_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_member_map_to_project_object() {
        let project_id = Uuid::new_v4();
        let project = resolve_authz_target("project", project_id).expect("project");
        assert_eq!(project.object_type, "project");
        assert_eq!(project.object_id, project_id);
        let member = resolve_authz_target("member", project_id).expect("member");
        assert_eq!(member.object_id, project_id);
    }

    #[test]
    fn generic_component_has_no_instance_target() {
        assert!(resolve_authz_target("component", Uuid::new_v4()).is_none());
    }

    #[test]
    fn generic_component_grant_maps_to_project_component_relation() {
        let project_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let read = exact_user_tuple("component", "read", project_id, user_id).expect("read");
        assert_eq!(read.object_type, "project");
        assert_eq!(read.object_id, project_id);
        assert_eq!(read.relation, "component_viewer");
        let write = exact_user_tuple("component", "write", project_id, user_id).expect("write");
        assert_eq!(write.relation, "component_editor");
    }

    #[test]
    fn uuid_and_prefixed_component_map_to_instance() {
        let project_id = Uuid::new_v4();
        let component_id = Uuid::new_v4();
        let naked = resolve_authz_target(&component_id.to_string(), project_id).expect("uuid");
        assert_eq!(naked.object_type, "component");
        assert_eq!(naked.object_id, component_id);
        let prefixed = resolve_authz_target(&format!("component:{component_id}"), project_id)
            .expect("prefixed");
        assert_eq!(prefixed.object_id, component_id);
    }

    #[test]
    fn component_write_is_editor_not_member() {
        assert_eq!(fga_relation("component", "write"), Some("editor"));
        assert_eq!(fga_relation("project", "write"), Some("member"));
        assert_eq!(fga_relation("component", "owner"), None);
    }
}
