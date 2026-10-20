//! Additive runtime identity and single-consumption journal (ADR-0012).
//!
//! Legacy enrollment uniqueness is deliberately untouched. No private keys,
//! projected tokens, CSR bodies, or invocation tickets are persisted here.
//! Reconciled with §§13.1–13.2: journal emission before forwarding, CAS accept.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r"
CREATE TABLE apparatus_runtime_enrollments (
    instance_id UUID PRIMARY KEY,
    project_id UUID NOT NULL,
    binding_id UUID NOT NULL,
    pod_uid TEXT NOT NULL UNIQUE CHECK (length(pod_uid) BETWEEN 1 AND 253),
    revision_uid TEXT NOT NULL CHECK (length(revision_uid) BETWEEN 1 AND 253),
    namespace TEXT NOT NULL CHECK (length(namespace) BETWEEN 1 AND 63),
    service_account TEXT NOT NULL CHECK (length(service_account) BETWEEN 1 AND 253),
    service_account_uid TEXT NOT NULL CHECK (length(service_account_uid) BETWEEN 1 AND 253),
    release TEXT NOT NULL,
    generation BIGINT NOT NULL CHECK (generation > 0),
    grant_revision BIGINT NOT NULL CHECK (grant_revision >= 0),
    revocation_epoch BIGINT NOT NULL CHECK (revocation_epoch >= 0),
    csr_sha256 TEXT NOT NULL CHECK (csr_sha256 ~ '^[0-9a-f]{64}$'),
    certificate_fingerprint TEXT NOT NULL UNIQUE
        CHECK (certificate_fingerprint ~ '^[0-9a-f]{64}$'),
    certificate_pem TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    closed_at TIMESTAMPTZ,
    CHECK (expires_at > issued_at AND expires_at <= issued_at + INTERVAL '5 minutes'),
    UNIQUE (project_id, binding_id, instance_id)
);
CREATE INDEX apparatus_runtime_enrollments_active
    ON apparatus_runtime_enrollments (project_id, binding_id, expires_at)
    WHERE closed_at IS NULL;

CREATE TABLE apparatus_runtime_executions (
    authorization_id UUID PRIMARY KEY,
    execution_id UUID UNIQUE,
    project_id UUID NOT NULL,
    binding_id UUID NOT NULL,
    instance_id UUID REFERENCES apparatus_runtime_enrollments(instance_id),
    source_instance_id UUID NOT NULL,
    source_binding_id UUID NOT NULL,
    generation BIGINT NOT NULL CHECK (generation > 0),
    grant_revision BIGINT NOT NULL CHECK (grant_revision >= 0),
    revocation_epoch BIGINT NOT NULL CHECK (revocation_epoch >= 0),
    operation_id UUID NOT NULL,
    operation_sha256 TEXT NOT NULL CHECK (operation_sha256 ~ '^[0-9a-f]{64}$'),
    issued_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    ticket_claims JSONB NOT NULL CHECK (jsonb_typeof(ticket_claims) = 'object'),
    execution_context JSONB,
    deadline TIMESTAMPTZ NOT NULL,
    retain_until TIMESTAMPTZ NOT NULL,
    closed_at TIMESTAMPTZ,
    CHECK (source_binding_id <> binding_id),
    CHECK (deadline > issued_at AND deadline <= issued_at + INTERVAL '30 seconds'),
    CHECK (accepted_at IS NULL OR (accepted_at >= issued_at AND accepted_at < deadline)),
    CHECK ((execution_id IS NULL AND instance_id IS NULL AND accepted_at IS NULL
            AND execution_context IS NULL)
        OR (execution_id IS NOT NULL AND instance_id IS NOT NULL AND accepted_at IS NOT NULL
            AND execution_context IS NOT NULL AND jsonb_typeof(execution_context) = 'object')),
    CHECK (retain_until >= deadline + INTERVAL '24 hours'),
    FOREIGN KEY (project_id, binding_id, instance_id)
        REFERENCES apparatus_runtime_enrollments(project_id, binding_id, instance_id)
);
CREATE INDEX apparatus_runtime_executions_active
    ON apparatus_runtime_executions (project_id, binding_id, deadline)
    WHERE closed_at IS NULL;
CREATE INDEX apparatus_runtime_executions_retention
    ON apparatus_runtime_executions (retain_until);

-- Negative directory installed before the writer's cutover (§13.3-B). It is
-- deliberately independent of legacy credentials and has no expiry or purge.
CREATE TABLE apparatus_runtime_cutover_fences (
    binding_id UUID PRIMARY KEY,
    migration_id UUID NOT NULL,
    project_id UUID NOT NULL,
    inventory_sha256 TEXT NOT NULL CHECK (inventory_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    installed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    observed_writer_cas JSONB CHECK (observed_writer_cas IS NULL
        OR jsonb_typeof(observed_writer_cas) = 'object')
);
CREATE INDEX apparatus_runtime_cutover_fences_project
    ON apparatus_runtime_cutover_fences (project_id, binding_id);
CREATE FUNCTION apparatus_runtime_preserve_cutover_fence() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'runtime_cutover_fence_is_irreversible' USING ERRCODE = '55000';
    END IF;
    IF NEW.binding_id IS DISTINCT FROM OLD.binding_id
       OR NEW.project_id IS DISTINCT FROM OLD.project_id
       OR NEW.migration_id IS DISTINCT FROM OLD.migration_id
       OR NEW.inventory_sha256 IS DISTINCT FROM OLD.inventory_sha256
       OR NEW.installed_at IS DISTINCT FROM OLD.installed_at THEN
        RAISE EXCEPTION 'runtime_cutover_fence_is_immutable' USING ERRCODE = '55000';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER apparatus_runtime_preserve_cutover_fence BEFORE UPDATE OR DELETE
    ON apparatus_runtime_cutover_fences FOR EACH ROW
    EXECUTE FUNCTION apparatus_runtime_preserve_cutover_fence();

-- Cross-process quota locks: collisions serialize, never authorize.
CREATE FUNCTION apparatus_runtime_enrollment_quota() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'runtime-enrollment:' || NEW.project_id::text || ':' || NEW.binding_id::text, 0));
    IF (SELECT count(*) FROM apparatus_runtime_enrollments
        WHERE project_id = NEW.project_id AND binding_id = NEW.binding_id
          AND closed_at IS NULL AND expires_at > clock_timestamp()) >= 1000 THEN
        RAISE EXCEPTION 'runtime_enrollment_quota' USING ERRCODE = '54000';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER apparatus_runtime_enrollment_quota BEFORE INSERT
    ON apparatus_runtime_enrollments FOR EACH ROW
    EXECUTE FUNCTION apparatus_runtime_enrollment_quota();

CREATE FUNCTION apparatus_runtime_execution_quota() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended(
        'runtime-execution:' || NEW.project_id::text || ':' || NEW.binding_id::text, 0));
    IF (SELECT count(*) FROM apparatus_runtime_executions
        WHERE project_id = NEW.project_id AND binding_id = NEW.binding_id
          AND closed_at IS NULL AND deadline > clock_timestamp()) >= 1000 THEN
        RAISE EXCEPTION 'runtime_execution_quota' USING ERRCODE = '54000';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER apparatus_runtime_execution_quota BEFORE INSERT
    ON apparatus_runtime_executions FOR EACH ROW
    EXECUTE FUNCTION apparatus_runtime_execution_quota();
        ",
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Tombstones and consumed authorizations must survive rollback.
        Err(DbErr::Migration(
            "runtime_control_rollback_requires_explicit_data_plan".to_owned(),
        ))
    }
}
