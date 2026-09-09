//! One-shot Manifesto DB → OpenFGA reconciliation.
//!
//! Reads live project memberships and visibility from Postgres and writes
//! the matching tuples without resetting the store.

use anyhow::{Context, Result};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

use crate::fga_client::{OpenFgaWriteClient, Tuple};
use manifesto_events::authz::fga_relation;

/// Desired OpenFGA tuple derived from Manifesto state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredTuple {
    pub object_type: String,
    pub object_id: Uuid,
    pub relation: String,
    pub user_id: Uuid,
}

impl DesiredTuple {
    fn to_fga(&self) -> Tuple {
        Tuple::user(
            self.object_type.clone(),
            self.object_id,
            self.relation.clone(),
            self.user_id,
        )
    }
}

/// Build the exact user tuples that should exist for an active grant.
#[must_use]
pub fn tuple_for_grant(
    resource: &str,
    permission: &str,
    project_id: Uuid,
    user_id: Uuid,
) -> Option<DesiredTuple> {
    let (object_type, object_id) = if let Ok(component_id) = Uuid::parse_str(resource) {
        ("component".to_string(), component_id)
    } else if let Some(("component", id)) = resource.split_once(':') {
        let component_id = Uuid::parse_str(id).ok()?;
        ("component".to_string(), component_id)
    } else if resource.eq_ignore_ascii_case("project")
        || resource.eq_ignore_ascii_case("member")
        || resource.eq_ignore_ascii_case("component")
    {
        if resource.eq_ignore_ascii_case("component") {
            return None;
        }
        ("project".to_string(), project_id)
    } else {
        return None;
    };
    let relation = fga_relation(&object_type, permission)?;
    Some(DesiredTuple {
        object_type,
        object_id,
        relation: relation.to_string(),
        user_id,
    })
}

/// Wildcard public-read tuple for a public active project.
#[must_use]
pub fn public_wildcard(project_id: Uuid) -> Tuple {
    Tuple::wildcard_user("project", project_id, "viewer")
}

/// Reconcile Manifesto rows into OpenFGA without deleting the store.
///
/// # Errors
///
/// Returns if the database or OpenFGA write fails.
pub async fn reconcile_manifesto(database_url: &str, fga: &OpenFgaWriteClient) -> Result<usize> {
    let db = Database::connect(database_url)
        .await
        .context("failed to connect to Manifesto database")?;
    let tuples = load_desired_tuples(&db).await?;
    let mut written = 0;
    for chunk in tuples.chunks(40) {
        fga.write_idempotent(chunk, &[]).await?;
        written += chunk.len();
    }
    Ok(written)
}

async fn load_desired_tuples(db: &DatabaseConnection) -> Result<Vec<Tuple>> {
    let grant_rows = db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            r"
SELECT pm.user_id, pm.project_id, r.name AS resource, p.level AS permission
FROM project_members pm
JOIN project_member_role_permissions pmr ON pmr.member_id = pm.id
JOIN role_permissions rp ON rp.id = pmr.role_permission_id
JOIN permissions p ON p.id = rp.permission_id
JOIN resources r ON r.id = rp.resource_id
WHERE pm.removed_at IS NULL
            "
            .to_string(),
        ))
        .await
        .context("failed to load membership grants")?;

    let mut tuples = Vec::new();
    for row in grant_rows {
        let user_id: Uuid = row.try_get("", "user_id")?;
        let project_id: Uuid = row.try_get("", "project_id")?;
        let resource: String = row.try_get("", "resource")?;
        let permission: String = row.try_get("", "permission")?;
        if let Some(desired) = tuple_for_grant(&resource, &permission, project_id, user_id) {
            tuples.push(desired.to_fga());
        }
    }

    let public_rows = db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            r"
SELECT id
FROM projects
WHERE visibility = 'public' AND status = 'active'
            "
            .to_string(),
        ))
        .await
        .context("failed to load public active projects")?;
    for row in public_rows {
        let project_id: Uuid = row.try_get("", "id")?;
        tuples.push(public_wildcard(project_id));
    }
    Ok(tuples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_read_becomes_viewer() {
        let project_id = Uuid::nil();
        let user_id = Uuid::from_u128(1);
        let tuple = tuple_for_grant("project", "read", project_id, user_id).expect("tuple");
        assert_eq!(tuple.object_type, "project");
        assert_eq!(tuple.relation, "viewer");
        assert_eq!(tuple.user_id, user_id);
    }

    #[test]
    fn generic_component_grant_is_skipped() {
        assert!(tuple_for_grant("component", "read", Uuid::nil(), Uuid::from_u128(1)).is_none());
    }

    #[test]
    fn instance_uuid_becomes_component_editor_for_write() {
        let component_id = Uuid::from_u128(9);
        let tuple = tuple_for_grant(
            &component_id.to_string(),
            "write",
            Uuid::nil(),
            Uuid::from_u128(1),
        )
        .expect("tuple");
        assert_eq!(tuple.object_type, "component");
        assert_eq!(tuple.object_id, component_id);
        assert_eq!(tuple.relation, "editor");
    }
}
