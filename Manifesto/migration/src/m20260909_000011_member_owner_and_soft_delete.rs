use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r"
ALTER TABLE project_members
    ADD COLUMN IF NOT EXISTS is_owner BOOLEAN NOT NULL DEFAULT false;
            ",
        )
        .await?;
        db.execute_unprepared(
            r"
UPDATE project_members SET is_owner = false;
            ",
        )
        .await?;
        db.execute_unprepared(
            r"
UPDATE project_members pm
SET is_owner = true
WHERE pm.removed_at IS NULL
  AND pm.id IN (
    SELECT DISTINCT ON (pm2.project_id) pm2.id
    FROM project_members pm2
    JOIN project_member_role_permissions pmr
      ON pmr.member_id = pm2.id
    JOIN role_permissions rp ON rp.id = pmr.role_permission_id
    JOIN permissions p ON p.id = rp.permission_id
    JOIN resources r ON r.id = rp.resource_id
    WHERE pm2.removed_at IS NULL
      AND p.level = 'owner'
      AND lower(r.name) = 'project'
    ORDER BY pm2.project_id, pm2.added_at ASC
  );
            ",
        )
        .await?;
        db.execute_unprepared(
            r"
UPDATE project_members pm
SET removed_at = NOW(),
    removal_reason = 'duplicate_active_membership'
WHERE pm.removed_at IS NULL
  AND pm.id NOT IN (
    SELECT keep_id FROM (
      SELECT DISTINCT ON (project_id, user_id) id AS keep_id
      FROM project_members
      WHERE removed_at IS NULL
      ORDER BY project_id, user_id, added_at ASC
    ) keepers
  );
            ",
        )
        .await?;
        db.execute_unprepared(r"DROP INDEX IF EXISTS project_members_unique;")
            .await?;
        db.execute_unprepared(
            r"
CREATE UNIQUE INDEX IF NOT EXISTS project_members_one_active
    ON project_members (project_id, user_id)
    WHERE removed_at IS NULL;
            ",
        )
        .await?;
        db.execute_unprepared(
            r"
CREATE UNIQUE INDEX IF NOT EXISTS project_members_one_owner
    ON project_members (project_id)
    WHERE is_owner AND removed_at IS NULL;
            ",
        )
        .await?;
        db.execute_unprepared(
            r"
CREATE INDEX IF NOT EXISTS idx_project_members_restorable
    ON project_members (project_id, user_id, grace_period_ends_at)
    WHERE removed_at IS NOT NULL;
            ",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(r"DROP INDEX IF EXISTS idx_project_members_restorable;")
            .await?;
        db.execute_unprepared(r"DROP INDEX IF EXISTS project_members_one_owner;")
            .await?;
        db.execute_unprepared(r"DROP INDEX IF EXISTS project_members_one_active;")
            .await?;
        db.execute_unprepared(
            r"
CREATE UNIQUE INDEX IF NOT EXISTS project_members_unique
    ON project_members (project_id, user_id);
            ",
        )
        .await?;
        db.execute_unprepared(r"ALTER TABLE project_members DROP COLUMN IF EXISTS is_owner;")
            .await?;
        Ok(())
    }
}
