//! Durable admission high-watermarks and one-use maintenance intentions. No
//! migration/backfill creates positive authority or enables a maintenance port.
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(SCHEMA).await?;
        Ok(())
    }
    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "admission fences require explicit retention decision".into(),
        ))
    }
}

const SCHEMA: &str = r"
CREATE TABLE apparatus_runtime_admission_guards (
    authority_id uuid NOT NULL,
    release text NOT NULL CHECK (release ~ '^sha256:[0-9a-f]{64}$'),
    record_id uuid,
    maximum_revision bigint NOT NULL DEFAULT 0 CHECK (maximum_revision >= 0),
    blocked boolean NOT NULL DEFAULT true,
    reconciliation_required boolean NOT NULL DEFAULT true,
    PRIMARY KEY (authority_id, release),
    CHECK ((maximum_revision = 0 AND record_id IS NULL) OR (maximum_revision >= 1 AND record_id IS NOT NULL))
);
CREATE TABLE apparatus_runtime_admission_challenges (
    challenge_id uuid PRIMARY KEY,
    authority_id uuid NOT NULL,
    installation_id uuid NOT NULL,
    migration_id uuid NOT NULL,
    project_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    action text NOT NULL CHECK (action IN ('prepare','open')),
    release text NOT NULL CHECK (release ~ '^sha256:[0-9a-f]{64}$'),
    intention jsonb NOT NULL CHECK (jsonb_typeof(intention) = 'object'),
    issued_at_unix_ms bigint NOT NULL CHECK (issued_at_unix_ms >= 0),
    expires_at_unix_ms bigint NOT NULL,
    invalidated boolean NOT NULL DEFAULT false,
    consumed_at_unix_ms bigint,
    CHECK (expires_at_unix_ms = issued_at_unix_ms + 60000),
    CHECK (consumed_at_unix_ms IS NULL OR consumed_at_unix_ms >= issued_at_unix_ms)
);
CREATE UNIQUE INDEX apparatus_runtime_admission_pending_intent
    ON apparatus_runtime_admission_challenges (migration_id, action)
    WHERE consumed_at_unix_ms IS NULL AND NOT invalidated;
CREATE TABLE apparatus_runtime_admission_journal (
    challenge_id uuid PRIMARY KEY REFERENCES apparatus_runtime_admission_challenges(challenge_id),
    jws_sha256 bytea NOT NULL UNIQUE CHECK (octet_length(jws_sha256) = 32),
    record_id uuid NOT NULL,
    admission_revision bigint NOT NULL CHECK (admission_revision >= 1),
    policy_id text NOT NULL CHECK (octet_length(policy_id) BETWEEN 1 AND 128),
    report_sha256 text NOT NULL CHECK (report_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    envelope_reference text NOT NULL CHECK (octet_length(envelope_reference) BETWEEN 1 AND 2048),
    cri_image text NOT NULL CHECK (octet_length(cri_image) BETWEEN 1 AND 2048),
    kid text NOT NULL,
    observed_at_unix_ms bigint NOT NULL,
    valid_until_unix_ms bigint NOT NULL,
    committed_at_unix_ms bigint NOT NULL,
    CHECK (observed_at_unix_ms <= committed_at_unix_ms AND committed_at_unix_ms < valid_until_unix_ms),
    CHECK (valid_until_unix_ms <= observed_at_unix_ms + 5000)
);
";
