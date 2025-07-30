use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ExternalProviders::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ExternalProviders::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_owned()),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::ProviderType)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::ConfigSchema)
                            .json_binary()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(ExternalProviders::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_providers_provider_type")
                    .table(ExternalProviders::Table)
                    .col(ExternalProviders::ProviderType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_external_providers_is_active")
                    .table(ExternalProviders::Table)
                    .col(ExternalProviders::IsActive)
                    .to_owned(),
            )
            .await?;

        // Insert default providers
        let github_schema = r#"{
            "type": "object",
            "properties": {
                "org_name": {
                    "type": "string",
                    "description": "GitHub organization name"
                },
                "access_token": {
                    "type": "string",
                    "description": "GitHub access token"
                },
                "base_url": {
                    "type": "string",
                    "description": "GitHub API base URL (for GitHub Enterprise)",
                    "default": "https://api.github.com"
                }
            },
            "required": ["org_name", "access_token"]
        }"#;

        let gitlab_schema = r#"{
            "type": "object",
            "properties": {
                "group_id": {
                    "type": "string",
                    "description": "GitLab group ID"
                },
                "access_token": {
                    "type": "string",
                    "description": "GitLab access token"
                },
                "base_url": {
                    "type": "string",
                    "description": "GitLab instance URL",
                    "default": "https://gitlab.com"
                }
            },
            "required": ["group_id", "access_token"]
        }"#;

        let confluence_schema = r#"{
            "type": "object",
            "properties": {
                "space_key": {
                    "type": "string",
                    "description": "Confluence space key"
                },
                "api_token": {
                    "type": "string",
                    "description": "Confluence API token"
                },
                "username": {
                    "type": "string",
                    "description": "Confluence username"
                },
                "base_url": {
                    "type": "string",
                    "description": "Confluence instance URL"
                }
            },
            "required": ["space_key", "api_token", "username", "base_url"]
        }"#;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(ExternalProviders::Table)
                    .columns([
                        ExternalProviders::ProviderType,
                        ExternalProviders::Name,
                        ExternalProviders::ConfigSchema,
                    ])
                    .values_panic(["github".into(), "GitHub".into(), github_schema.into()])
                    .values_panic(["gitlab".into(), "GitLab".into(), gitlab_schema.into()])
                    .values_panic([
                        "confluence".into(),
                        "Confluence".into(),
                        confluence_schema.into(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ExternalProviders::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ExternalProviders {
    Table,
    Id,
    ProviderType,
    Name,
    ConfigSchema,
    IsActive,
    CreatedAt,
}
