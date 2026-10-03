//! One-shot Manifesto DB → OpenFGA reconciliation.
//!
//! Reads live project memberships, owners, parents, grants and visibility
//! from Postgres, compares them with existing `project`/`component` tuples,
//! and applies the exact write/delete delta without resetting the store.

use std::collections::HashSet;

use anyhow::{Context, Result};
use manifesto_events::authz::exact_user_tuple;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

use crate::fga_client::{OpenFgaWriteClient, Tuple};

const PAGE_SIZE: u64 = 500;
const WRITE_CHUNK: usize = 40;

/// Build the exact user tuples that should exist for an active grant.
#[must_use]
pub fn tuple_for_grant(
    resource: &str,
    permission: &str,
    project_id: Uuid,
    user_id: Uuid,
) -> Option<Tuple> {
    exact_user_tuple(resource, permission, project_id, user_id).map(|tuple| {
        Tuple::user(
            tuple.object_type,
            tuple.object_id,
            tuple.relation,
            tuple.user_id,
        )
    })
}

/// Wildcard public-read tuple for a public active project.
#[must_use]
pub fn public_wildcard(project_id: Uuid) -> Tuple {
    Tuple::wildcard_user("project", project_id, "viewer")
}

/// Compute writes = desired − existing and deletes = existing − desired.
#[must_use]
pub fn diff_tuples(desired: &[Tuple], existing: &[Tuple]) -> (Vec<Tuple>, Vec<Tuple>) {
    let desired: HashSet<Tuple> = desired.iter().cloned().collect();
    let existing: HashSet<Tuple> = existing.iter().cloned().collect();
    (
        desired.difference(&existing).cloned().collect(),
        existing.difference(&desired).cloned().collect(),
    )
}

/// Reconcile Manifesto rows into OpenFGA without deleting the store.
///
/// Only `project` and `component` tuples are compared. Other types are left
/// untouched. Missing desired tuples are written; leftover Manifesto tuples
/// are deleted.
///
/// # Errors
///
/// Returns if the database or OpenFGA write fails.
pub async fn reconcile_manifesto(database_url: &str, fga: &OpenFgaWriteClient) -> Result<usize> {
    let db = Database::connect(database_url)
        .await
        .context("failed to connect to Manifesto database")?;
    let desired = load_desired_tuples(&db).await?;
    let existing = fga.read_manifesto_tuples().await?;
    let (writes, deletes) = diff_tuples(&desired, &existing);
    let mut applied = 0;
    for chunk in writes.chunks(WRITE_CHUNK) {
        fga.write_idempotent(chunk, &[]).await?;
        applied += chunk.len();
    }
    for chunk in deletes.chunks(WRITE_CHUNK) {
        fga.write_idempotent(&[], chunk).await?;
        applied += chunk.len();
    }
    Ok(applied)
}

async fn load_desired_tuples(db: &DatabaseConnection) -> Result<Vec<Tuple>> {
    let mut tuples = HashSet::new();
    load_grant_tuples(db, &mut tuples).await?;
    load_owner_tuples(db, &mut tuples).await?;
    load_project_parent_tuples(db, &mut tuples).await?;
    load_visibility_tuples(db, &mut tuples).await?;
    load_component_parent_tuples(db, &mut tuples).await?;
    Ok(tuples.into_iter().collect())
}

async fn paginated_query(db: &DatabaseConnection, sql: &str) -> Result<Vec<sea_orm::QueryResult>> {
    let mut offset = 0_u64;
    let mut rows = Vec::new();
    loop {
        let page = db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                &format!("{sql} LIMIT {PAGE_SIZE} OFFSET {offset}"),
                [],
            ))
            .await?;
        let count = page.len();
        rows.extend(page);
        if count < PAGE_SIZE as usize {
            break;
        }
        offset += PAGE_SIZE;
    }
    Ok(rows)
}

async fn load_grant_tuples(db: &DatabaseConnection, tuples: &mut HashSet<Tuple>) -> Result<()> {
    let rows = paginated_query(
        db,
        r"
SELECT pm.user_id, pm.project_id, r.name AS resource, p.level AS permission
FROM project_members pm
JOIN project_member_role_permissions pmr ON pmr.member_id = pm.id
JOIN role_permissions rp ON rp.id = pmr.role_permission_id
JOIN permissions p ON p.id = rp.permission_id
JOIN resources r ON r.id = rp.resource_id
WHERE pm.removed_at IS NULL
ORDER BY pm.id, r.name, p.level
        ",
    )
    .await
    .context("failed to load membership grants")?;
    for row in rows {
        let user_id: Uuid = row.try_get("", "user_id")?;
        let project_id: Uuid = row.try_get("", "project_id")?;
        let resource: String = row.try_get("", "resource")?;
        let permission: String = row.try_get("", "permission")?;
        if let Some(tuple) = tuple_for_grant(&resource, &permission, project_id, user_id) {
            tuples.insert(tuple);
        }
    }
    Ok(())
}

async fn load_owner_tuples(db: &DatabaseConnection, tuples: &mut HashSet<Tuple>) -> Result<()> {
    let rows = paginated_query(
        db,
        r"
SELECT user_id, project_id
FROM project_members
WHERE is_owner AND removed_at IS NULL
ORDER BY project_id
        ",
    )
    .await
    .context("failed to load project owners")?;
    for row in rows {
        let user_id: Uuid = row.try_get("", "user_id")?;
        let project_id: Uuid = row.try_get("", "project_id")?;
        tuples.insert(Tuple::user("project", project_id, "owner", user_id));
    }
    Ok(())
}

async fn load_project_parent_tuples(
    db: &DatabaseConnection,
    tuples: &mut HashSet<Tuple>,
) -> Result<()> {
    let rows = paginated_query(
        db,
        r"
SELECT id, owner_id
FROM projects
WHERE owner_type = 'organization'
ORDER BY id
        ",
    )
    .await
    .context("failed to load organization parents")?;
    for row in rows {
        let project_id: Uuid = row.try_get("", "id")?;
        let owner_id: Uuid = row.try_get("", "owner_id")?;
        tuples.insert(Tuple::object(
            "project",
            project_id,
            "organization",
            "organization",
            owner_id,
        ));
    }
    Ok(())
}

async fn load_visibility_tuples(
    db: &DatabaseConnection,
    tuples: &mut HashSet<Tuple>,
) -> Result<()> {
    let rows = paginated_query(
        db,
        r"
SELECT id, visibility, status, owner_type, owner_id
FROM projects
ORDER BY id
        ",
    )
    .await
    .context("failed to load project visibility")?;
    for row in rows {
        let project_id: Uuid = row.try_get("", "id")?;
        let visibility: String = row.try_get("", "visibility")?;
        let status: String = row.try_get("", "status")?;
        let owner_type: String = row.try_get("", "owner_type")?;
        let owner_id: Uuid = row.try_get("", "owner_id")?;
        let live = status.eq_ignore_ascii_case("active");
        if visibility.eq_ignore_ascii_case("public") && live {
            tuples.insert(public_wildcard(project_id));
        }
        if visibility.eq_ignore_ascii_case("internal")
            && live
            && owner_type.eq_ignore_ascii_case("organization")
        {
            tuples.insert(Tuple::userset(
                "project",
                project_id,
                "viewer",
                "organization",
                owner_id,
                "member",
            ));
        }
    }
    Ok(())
}

async fn load_component_parent_tuples(
    db: &DatabaseConnection,
    tuples: &mut HashSet<Tuple>,
) -> Result<()> {
    let rows = paginated_query(
        db,
        r"
SELECT id, project_id
FROM project_components
ORDER BY id
        ",
    )
    .await
    .context("failed to load component parents")?;
    for row in rows {
        let component_id: Uuid = row.try_get("", "id")?;
        let project_id: Uuid = row.try_get("", "project_id")?;
        tuples.insert(Tuple::object(
            "component",
            component_id,
            "project",
            "project",
            project_id,
        ));
    }
    Ok(())
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
        assert_eq!(tuple.user_id, user_id.to_string());
    }

    #[test]
    fn generic_component_grant_becomes_component_viewer() {
        let tuple =
            tuple_for_grant("component", "read", Uuid::nil(), Uuid::from_u128(1)).expect("tuple");
        assert_eq!(tuple.object_type, "project");
        assert_eq!(tuple.relation, "component_viewer");
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
        assert_eq!(tuple.object_id, component_id.to_string());
        assert_eq!(tuple.relation, "editor");
    }

    #[test]
    fn diff_writes_missing_and_deletes_leftover() {
        let project_id = Uuid::from_u128(3);
        let desired = vec![Tuple::wildcard_user("project", project_id, "viewer")];
        let leftover = Tuple::user("project", project_id, "viewer", Uuid::from_u128(8));
        let (writes, deletes) = diff_tuples(&desired, &[leftover.clone()]);
        assert_eq!(writes, desired);
        assert_eq!(deletes, vec![leftover]);
    }

    #[test]
    fn diff_does_not_touch_already_matching_tuples() {
        let project_id = Uuid::from_u128(3);
        let tuple = Tuple::wildcard_user("project", project_id, "viewer");
        let (writes, deletes) = diff_tuples(&[tuple.clone()], &[tuple]);
        assert!(writes.is_empty());
        assert!(deletes.is_empty());
    }
}
