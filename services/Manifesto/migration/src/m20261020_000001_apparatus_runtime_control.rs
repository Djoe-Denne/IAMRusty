//! Additive runtime writer state. No legacy binding is activated by migration.
//! Tombstones deliberately have no cascading FK to components/projects.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(SCHEMA).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Removing durable fences/tombstones can resurrect replayed work. A
        // destructive rollback requires a separate operator-approved procedure.
        Err(DbErr::Custom(
            "runtime control rollback requires explicit retention decision".into(),
        ))
    }
}

const SCHEMA: &str = r"
CREATE TABLE apparatus_runtime_bindings (
    project_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation >= 0),
    grant_revision bigint NOT NULL CHECK (grant_revision >= 0),
    revocation_epoch bigint NOT NULL CHECK (revocation_epoch >= 0),
    projection_version bigint NOT NULL DEFAULT 1 CHECK (projection_version >= 1),
    state text NOT NULL DEFAULT 'closed' CHECK (state IN ('closed','active','draining','tombstone')),
    release text NOT NULL CHECK (release ~ '^sha256:[0-9a-f]{64}$'),
    profile text NOT NULL,
    cutover_migration_id uuid,
    cutover_phase text CHECK (cutover_phase IN ('fenced','prepared','opened')),
    cutover_plan jsonb CHECK (jsonb_typeof(cutover_plan)='object'),
    cutover_journal jsonb NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(cutover_journal)='array'),
    profile_lock_sha256 text CHECK (profile_lock_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    inventory_sha256 text CHECK (inventory_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    drain_evidence_sha256 text CHECK (drain_evidence_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    owned_service_uid text CHECK (octet_length(owned_service_uid) BETWEEN 1 AND 128),
    writer_fence bigint NOT NULL DEFAULT 0 CHECK (writer_fence >= 0),
    lease_id uuid,
    lease_holder uuid,
    lease_principal text,
    lease_issued_at_ms bigint,
    lease_expires_at_ms bigint,
    updated_at_ms bigint NOT NULL CHECK (updated_at_ms >= 0),
    PRIMARY KEY (project_id, binding_id),
    CHECK ((lease_id IS NULL AND lease_holder IS NULL AND lease_principal IS NULL
            AND lease_issued_at_ms IS NULL AND lease_expires_at_ms IS NULL)
        OR (lease_id IS NOT NULL AND lease_holder IS NOT NULL AND lease_principal IS NOT NULL
            AND lease_issued_at_ms IS NOT NULL AND lease_expires_at_ms IS NOT NULL
            AND lease_issued_at_ms >= 0 AND lease_expires_at_ms > lease_issued_at_ms
            AND lease_expires_at_ms - lease_issued_at_ms = 10000 AND writer_fence > 0))
);
CREATE INDEX apparatus_runtime_binding_state_idx ON apparatus_runtime_bindings(project_id, state, binding_id);

CREATE TABLE apparatus_runtime_observations (
    project_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    generation bigint NOT NULL CHECK (generation >= 0),
    grant_revision bigint NOT NULL CHECK (grant_revision >= 0),
    revocation_epoch bigint NOT NULL CHECK (revocation_epoch >= 0),
    projection_version bigint NOT NULL CHECK (projection_version >= 1),
    writer_fence bigint NOT NULL CHECK (writer_fence > 0),
    service_uid text NOT NULL CHECK (octet_length(service_uid) BETWEEN 1 AND 128),
    resource_generation bigint NOT NULL CHECK (resource_generation >= 0),
    observed_resource_generation bigint NOT NULL CHECK (observed_resource_generation >= 0),
    revision_uid text NOT NULL CHECK (octet_length(revision_uid) BETWEEN 1 AND 128),
    revision_name text NOT NULL CHECK (length(revision_name) BETWEEN 1 AND 253),
    endpoint text NOT NULL CHECK (length(endpoint) BETWEEN 1 AND 2048),
    release text NOT NULL CHECK (release ~ '^sha256:[0-9a-f]{64}$'),
    route_ready boolean NOT NULL,
    observed_at_ms bigint NOT NULL CHECK (observed_at_ms >= 0),
    PRIMARY KEY (project_id, binding_id),
    FOREIGN KEY (project_id,binding_id) REFERENCES apparatus_runtime_bindings(project_id,binding_id) ON DELETE RESTRICT
);

CREATE TABLE apparatus_runtime_instances (
    project_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    instance_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('pod','legacy_fence')),
    state text NOT NULL CHECK (state IN ('current','closed','tombstone')),
    generation bigint NOT NULL CHECK (generation >= 0),
    grant_revision bigint NOT NULL CHECK (grant_revision >= 0),
    revocation_epoch bigint NOT NULL CHECK (revocation_epoch >= 0),
    projection_version bigint NOT NULL CHECK (projection_version >= 1),
    writer_fence bigint CHECK (writer_fence > 0),
    pod_uid text CHECK (octet_length(pod_uid) BETWEEN 1 AND 128),
    revision_uid text CHECK (octet_length(revision_uid) BETWEEN 1 AND 128),
    namespace text CHECK (length(namespace) BETWEEN 1 AND 63),
    service_account text CHECK (length(service_account) BETWEEN 1 AND 253),
    service_account_uid text CHECK (octet_length(service_account_uid) BETWEEN 1 AND 128),
    release text NOT NULL CHECK (release ~ '^sha256:[0-9a-f]{64}$'),
    observed_at_ms bigint CHECK (observed_at_ms >= 0),
    valid_until_ms bigint CHECK (valid_until_ms > observed_at_ms AND valid_until_ms <= observed_at_ms + 5000),
    closed_at_ms bigint CHECK (closed_at_ms >= 0),
    PRIMARY KEY (project_id,binding_id,instance_id),
    UNIQUE (project_id,binding_id,pod_uid),
    CHECK ((state='current' AND closed_at_ms IS NULL)
        OR (state IN ('closed','tombstone') AND closed_at_ms IS NOT NULL)),
    CHECK ((kind='pod' AND pod_uid IS NOT NULL AND revision_uid IS NOT NULL
            AND namespace IS NOT NULL AND service_account IS NOT NULL AND service_account_uid IS NOT NULL
            AND writer_fence IS NOT NULL AND observed_at_ms IS NOT NULL AND valid_until_ms IS NOT NULL)
        OR (kind='legacy_fence' AND state='tombstone' AND pod_uid IS NULL AND revision_uid IS NULL
            AND namespace IS NULL AND service_account IS NULL AND service_account_uid IS NULL
            AND writer_fence IS NULL AND observed_at_ms IS NULL AND valid_until_ms IS NULL)),
    FOREIGN KEY (project_id,binding_id) REFERENCES apparatus_runtime_bindings(project_id,binding_id) ON DELETE RESTRICT
);
CREATE INDEX apparatus_runtime_instance_live_idx ON apparatus_runtime_instances(project_id,binding_id,state);

CREATE TABLE apparatus_runtime_effects (
    project_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    execution_id uuid NOT NULL,
    effect_id uuid NOT NULL,
    instance_id uuid NOT NULL,
    authorization_id uuid NOT NULL,
    execution_context jsonb NOT NULL CHECK (jsonb_typeof(execution_context)='object'),
    source_binding_id uuid NOT NULL,
    source_instance_id uuid NOT NULL,
    source_revocation_epoch bigint NOT NULL CHECK (source_revocation_epoch >= 0),
    target_revocation_epoch bigint NOT NULL CHECK (target_revocation_epoch >= 0),
    grant_revision bigint NOT NULL CHECK (grant_revision >= 0),
    operation_sha256 text NOT NULL CHECK (operation_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    effect_sha256 text NOT NULL CHECK (effect_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    deadline_ms bigint NOT NULL CHECK (deadline_ms >= 0),
    begun_at_ms bigint NOT NULL CHECK (begun_at_ms >= 0 AND begun_at_ms < deadline_ms),
    completed_at_ms bigint CHECK (completed_at_ms >= begun_at_ms),
    completion_state text CHECK (completion_state IN ('completed','indeterminate')),
    PRIMARY KEY (project_id,binding_id,effect_id),
    FOREIGN KEY (project_id,binding_id,instance_id) REFERENCES apparatus_runtime_instances(project_id,binding_id,instance_id) ON DELETE RESTRICT,
    CHECK ((completed_at_ms IS NULL) = (completion_state IS NULL))
);
CREATE INDEX apparatus_runtime_effect_execution_idx ON apparatus_runtime_effects(project_id,binding_id,execution_id);
CREATE INDEX apparatus_runtime_effect_retention_idx ON apparatus_runtime_effects(deadline_ms);
";
