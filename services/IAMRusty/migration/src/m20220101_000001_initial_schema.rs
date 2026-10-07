use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_users_table(manager).await?;
        create_user_emails_table(manager).await?;
        create_provider_tokens_table(manager).await?;
        create_refresh_tokens_table(manager).await?;
        create_user_email_verification_table(manager).await?;
        create_password_reset_tokens_table(manager).await?;
        create_signing_keys_table(manager).await?;
        create_signing_scope_epochs(manager).await?;
        create_prepared_signing_publication(manager).await?;
        create_identities_table(manager).await?;
        create_auth_transactions(manager).await?;
        rustycog::outbox::outbox_migration().up(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rustycog::outbox::outbox_migration().down(manager).await?;
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE oauth_transactions")
            .await?;
        manager
            .drop_table(Table::drop().table(Identities::Table).to_owned())
            .await?;
        db.execute_unprepared("DROP TABLE signing_key_prepublications")
            .await?;
        db.execute_unprepared("DROP TABLE signing_jwks_publication")
            .await?;
        db.execute_unprepared("DROP TABLE signing_public_entries")
            .await?;
        db.execute_unprepared("DROP FUNCTION iam_invalidate_signing_publication()")
            .await?;
        manager
            .drop_table(Table::drop().table(SigningKeys::Table).to_owned())
            .await?;
        db.execute_unprepared("DROP FUNCTION iam_signing_admitted_at_immutable()")
            .await?;
        // Dropping signing_keys above removes its dependent epoch trigger first.
        db.execute_unprepared("DROP FUNCTION iam_advance_signing_scope_epoch()")
            .await?;
        db.execute_unprepared("DROP TABLE signing_scope_epochs")
            .await?;
        manager
            .drop_table(Table::drop().table(PasswordResetTokens::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserEmailVerification::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RefreshTokens::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ProviderTokens::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(UserEmails::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

/// Additive initial-source foundation for the later atomic writer cutover.
/// No migration of an already-migrated DB, PEM backfill, or data reset occurs.
async fn create_prepared_signing_publication(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    db.execute_unprepared(r#"
        CREATE TABLE signing_public_entries (
            signing_key_id uuid PRIMARY KEY REFERENCES signing_keys(id),
            public_n varchar(1366) NOT NULL,
            public_e varchar(11) NOT NULL,
            binding_fingerprint bytea NOT NULL CHECK (octet_length(binding_fingerprint)=32),
            longest_entry_bytes integer NOT NULL CHECK (longest_entry_bytes>0 AND longest_entry_bytes<=4096)
        )
    "#).await?;
    db.execute_unprepared(
        r#"
        CREATE TABLE signing_jwks_publication (
            singleton smallint PRIMARY KEY CHECK (singleton=1),
            revision bigint NOT NULL CHECK (revision>=0),
            dirty boolean NOT NULL DEFAULT true,
            payload text NOT NULL CHECK (octet_length(payload)<=786432),
            as_of timestamptz NOT NULL,
            next_expiration timestamptz NULL,
            access_token_ttl bigint NOT NULL CHECK (access_token_ttl>=0),
            retire_skew bigint NOT NULL CHECK (retire_skew=60)
        )
    "#,
    )
    .await?;
    // TTL0 is deliberately unmaterialized policy, never an inferred runtime TTL.
    // The cutover reader must refresh policy/clock before using this initial row.
    db.execute_unprepared(r#"
        INSERT INTO signing_jwks_publication(singleton,revision,payload,as_of,next_expiration,access_token_ttl,retire_skew)
        VALUES (1,0,'{"keys":[]}',statement_timestamp(),NULL,0,60)
    "#).await?;
    db.execute_unprepared(r#"
        CREATE INDEX signing_keys_slot_publication ON signing_keys(status,trust_scope,organization_id,updated_at)
        WHERE status IN ('pending','active','retiring')
    "#).await?;
    db.execute_unprepared(
        r#"
        CREATE INDEX signing_keys_slot_owner ON signing_keys(organization_id,status,updated_at)
        WHERE status IN ('pending','active','retiring')
    "#,
    )
    .await?;
    // Prepared records are immutable through the application. Independent SQL
    // corruption/removal must invalidate publication rather than leave a fresh
    // cached payload concealing the corrupt record indefinitely.
    db.execute_unprepared("CREATE FUNCTION iam_invalidate_signing_publication() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN UPDATE signing_jwks_publication SET dirty=true WHERE singleton=1; IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW; END $$").await?;
    db.execute_unprepared("CREATE TRIGGER signing_public_entries_publication_dirty AFTER INSERT OR UPDATE OR DELETE ON signing_public_entries FOR EACH ROW EXECUTE FUNCTION iam_invalidate_signing_publication()").await?;
    Ok(())
}

async fn create_users_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Users::Table)
                .if_not_exists()
                .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(Users::Username).string().null())
                .col(ColumnDef::new(Users::AvatarUrl).string())
                .col(ColumnDef::new(Users::PasswordHash).string().null())
                .col(
                    ColumnDef::new(Users::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Users::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await
}

async fn create_user_emails_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserEmails::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(UserEmails::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(UserEmails::UserId).uuid().not_null())
                .col(ColumnDef::new(UserEmails::Email).string().not_null())
                .col(
                    ColumnDef::new(UserEmails::IsPrimary)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(UserEmails::IsVerified)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(
                    ColumnDef::new(UserEmails::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(UserEmails::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_emails_user_id")
                        .from(UserEmails::Table, UserEmails::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::NoAction),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_user_emails_email_unique")
                .table(UserEmails::Table)
                .col(UserEmails::Email)
                .unique()
                .to_owned(),
        )
        .await
}

async fn create_provider_tokens_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ProviderTokens::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(ProviderTokens::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(ProviderTokens::UserId).uuid().not_null())
                .col(ColumnDef::new(ProviderTokens::Provider).string().not_null())
                .col(
                    ColumnDef::new(ProviderTokens::ProviderUserId)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ProviderTokens::AccessToken)
                        .string()
                        .not_null(),
                )
                .col(ColumnDef::new(ProviderTokens::RefreshToken).string())
                .col(ColumnDef::new(ProviderTokens::ExpiresIn).integer())
                .col(
                    ColumnDef::new(ProviderTokens::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(ProviderTokens::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-provider_tokens-user_id")
                        .from(ProviderTokens::Table, ProviderTokens::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_provider_tokens_provider_user_unique")
                .table(ProviderTokens::Table)
                .col(ProviderTokens::Provider)
                .col(ProviderTokens::ProviderUserId)
                .unique()
                .to_owned(),
        )
        .await
}

async fn create_refresh_tokens_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(RefreshTokens::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(RefreshTokens::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(RefreshTokens::UserId).uuid().not_null())
                .col(ColumnDef::new(RefreshTokens::Token).text().not_null())
                .col(
                    ColumnDef::new(RefreshTokens::IsValid)
                        .boolean()
                        .not_null()
                        .default(true),
                )
                .col(
                    ColumnDef::new(RefreshTokens::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(RefreshTokens::ExpiresAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-refresh_tokens-user_id")
                        .from(RefreshTokens::Table, RefreshTokens::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_user_email_verification_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserEmailVerification::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(UserEmailVerification::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(UserEmailVerification::Email)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(UserEmailVerification::VerificationToken)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(UserEmailVerification::ExpiresAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UserEmailVerification::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_email_verification_token")
                .table(UserEmailVerification::Table)
                .col(UserEmailVerification::VerificationToken)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_email_verification_expires_at")
                .table(UserEmailVerification::Table)
                .col(UserEmailVerification::ExpiresAt)
                .to_owned(),
        )
        .await
}

async fn create_password_reset_tokens_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PasswordResetTokens::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(PasswordResetTokens::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(PasswordResetTokens::UserId)
                        .uuid()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PasswordResetTokens::TokenHash)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(PasswordResetTokens::ExpiresAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(PasswordResetTokens::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(PasswordResetTokens::UsedAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_password_reset_tokens_user_id")
                        .from(PasswordResetTokens::Table, PasswordResetTokens::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_password_reset_tokens_user_id")
                .table(PasswordResetTokens::Table)
                .col(PasswordResetTokens::UserId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_password_reset_tokens_token_hash")
                .table(PasswordResetTokens::Table)
                .col(PasswordResetTokens::TokenHash)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_password_reset_tokens_expires_at")
                .table(PasswordResetTokens::Table)
                .col(PasswordResetTokens::ExpiresAt)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .if_not_exists()
                .name("idx_password_reset_tokens_used_at")
                .table(PasswordResetTokens::Table)
                .col(PasswordResetTokens::UsedAt)
                .to_owned(),
        )
        .await
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    AvatarUrl,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserEmails {
    Table,
    Id,
    UserId,
    Email,
    IsPrimary,
    IsVerified,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ProviderTokens {
    Table,
    Id,
    UserId,
    Provider,
    ProviderUserId,
    AccessToken,
    RefreshToken,
    ExpiresIn,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    Token,
    IsValid,
    CreatedAt,
    ExpiresAt,
}

#[derive(DeriveIden)]
enum UserEmailVerification {
    Table,
    Id,
    Email,
    VerificationToken,
    ExpiresAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum PasswordResetTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    CreatedAt,
    UsedAt,
}

async fn create_signing_keys_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SigningKeys::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(SigningKeys::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SigningKeys::Kid)
                        .string()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(SigningKeys::Algorithm).string().not_null())
                .col(ColumnDef::new(SigningKeys::TrustScope).string().not_null())
                .col(ColumnDef::new(SigningKeys::Issuer).string().not_null())
                .col(
                    ColumnDef::new(SigningKeys::ProviderType)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SigningKeys::ProviderKeyRef)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SigningKeys::ProviderKeyVersion)
                        .big_integer()
                        .null(),
                )
                .col(ColumnDef::new(SigningKeys::CredentialRef).string().null())
                .col(ColumnDef::new(SigningKeys::PublicKey).text().not_null())
                .col(ColumnDef::new(SigningKeys::Status).string().not_null())
                .col(ColumnDef::new(SigningKeys::OrganizationId).uuid().null())
                .col(
                    ColumnDef::new(SigningKeys::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(SigningKeys::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(SigningKeys::LifecycleAdmittedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::cust("statement_timestamp()")),
                )
                .to_owned(),
        )
        .await?;
    let db = manager.get_connection();
    db.execute_unprepared("CREATE INDEX signing_keys_organization_admitted_at ON signing_keys(organization_id,lifecycle_admitted_at)").await?;
    // Initial schema for NEW databases only; this is not an upgrade/backfill.
    db.execute_unprepared("ALTER TABLE signing_keys ADD CONSTRAINT signing_keys_provider_version CHECK ((provider_key_version IS NULL OR provider_key_version BETWEEN 1 AND 4294967295) AND (provider_type <> 'openbao_transit' OR provider_key_version IS NOT NULL))").await?;
    db.execute_unprepared("CREATE FUNCTION iam_signing_admitted_at_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.lifecycle_admitted_at IS DISTINCT FROM OLD.lifecycle_admitted_at THEN RAISE EXCEPTION 'signing admission history is immutable'; END IF; RETURN NEW; END $$").await?;
    db.execute_unprepared("CREATE TRIGGER signing_keys_admission_immutable BEFORE UPDATE ON signing_keys FOR EACH ROW EXECUTE FUNCTION iam_signing_admitted_at_immutable()").await?;
    db.execute_unprepared("CREATE UNIQUE INDEX signing_keys_one_active_platform ON signing_keys(trust_scope) WHERE trust_scope='platform' AND status='active'").await?;
    db.execute_unprepared("CREATE UNIQUE INDEX signing_keys_one_active_org ON signing_keys(organization_id) WHERE trust_scope='organization' AND status='active'").await?;
    Ok(())
}

async fn create_signing_scope_epochs(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    // NEW databases only. Never infer a provider version or migrate live rows.
    db.execute_unprepared("CREATE TABLE signing_scope_epochs (scope_id text PRIMARY KEY, revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0))").await?;
    db.execute_unprepared("CREATE TABLE signing_key_prepublications (key_id uuid PRIMARY KEY, scope_id text NOT NULL, revision bigint NOT NULL CHECK (revision > 0), previous_active_kid text NULL)").await?;
    db.execute_unprepared(
        "CREATE INDEX signing_prepublication_scope ON signing_key_prepublications(scope_id)",
    )
    .await?;
    // All existing writers participate, including low-level bootstrap/status APIs.
    // Transaction rollback rolls back the epoch; timestamps are never epochs.
    db.execute_unprepared("CREATE FUNCTION iam_advance_signing_scope_epoch() RETURNS trigger LANGUAGE plpgsql AS $$ DECLARE sid text; BEGIN sid := CASE WHEN NEW.trust_scope='platform' AND NEW.organization_id IS NULL THEN 'platform' WHEN NEW.trust_scope='organization' AND NEW.organization_id IS NOT NULL THEN 'organization:' || NEW.organization_id::text ELSE NULL END; IF sid IS NULL THEN RAISE EXCEPTION 'invalid signing scope'; END IF; UPDATE signing_jwks_publication SET dirty=true WHERE singleton=1; INSERT INTO signing_scope_epochs(scope_id,revision) VALUES(sid,1) ON CONFLICT(scope_id) DO UPDATE SET revision=signing_scope_epochs.revision+1; RETURN NEW; END $$").await?;
    db.execute_unprepared("CREATE TRIGGER signing_keys_scope_epoch AFTER INSERT OR UPDATE ON signing_keys FOR EACH ROW EXECUTE FUNCTION iam_advance_signing_scope_epoch()").await?;
    Ok(())
}

async fn create_identities_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Identities::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(Identities::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(Identities::UserId).uuid().not_null())
                .col(ColumnDef::new(Identities::Issuer).string().not_null())
                .col(ColumnDef::new(Identities::Subject).string().not_null())
                .col(ColumnDef::new(Identities::Kind).string().not_null())
                .col(
                    ColumnDef::new(Identities::CreatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(Identities::UpdatedAt)
                        .timestamp()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_identities_user_id")
                        .from(Identities::Table, Identities::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_identities_issuer_subject")
                .table(Identities::Table)
                .col(Identities::Issuer)
                .col(Identities::Subject)
                .unique()
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_identities_user_id")
                .table(Identities::Table)
                .col(Identities::UserId)
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_identities_user_issuer_kind")
                .table(Identities::Table)
                .col(Identities::UserId)
                .col(Identities::Issuer)
                .col(Identities::Kind)
                .unique()
                .to_owned(),
        )
        .await?;
    Ok(())
}

async fn create_auth_transactions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let db = manager.get_connection();
    db.execute_unprepared(
        "CREATE TABLE oauth_transactions (
            id UUID PRIMARY KEY,
            nonce_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(nonce_hash)=32),
            state_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(state_hash)=32),
            browser_nonce_hash BYTEA NOT NULL CHECK (octet_length(browser_nonce_hash)=32),
            provider VARCHAR(50) NOT NULL,
            operation VARCHAR(8) NOT NULL CHECK (operation IN ('login','link','relink')),
            target_user_id UUID REFERENCES users(id) ON DELETE CASCADE,
            redirect_uri TEXT NOT NULL,
            pkce_verifier TEXT,
            expires_at TIMESTAMPTZ NOT NULL,
            consumed_at TIMESTAMPTZ,
            CHECK ((operation = 'login' AND target_user_id IS NULL) OR
                   (operation IN ('link','relink') AND target_user_id IS NOT NULL))
        )",
    )
    .await?;
    db.execute_unprepared(
        "CREATE INDEX oauth_transactions_expiry ON oauth_transactions(expires_at)",
    )
    .await?;
    Ok(())
}

#[derive(Iden)]
enum SigningKeys {
    Table,
    Id,
    Kid,
    Algorithm,
    TrustScope,
    Issuer,
    ProviderType,
    ProviderKeyRef,
    ProviderKeyVersion,
    CredentialRef,
    PublicKey,
    Status,
    OrganizationId,
    CreatedAt,
    UpdatedAt,
    LifecycleAdmittedAt,
}

#[derive(Iden)]
enum Identities {
    Table,
    Id,
    UserId,
    Issuer,
    Subject,
    Kind,
    CreatedAt,
    UpdatedAt,
}
