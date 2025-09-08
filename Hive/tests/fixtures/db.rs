use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

use hive_infra::repository::entity::{
    organization_member_role_permissions,
    organization_members,
    organizations,
    permissions,
    resources,
    role_permissions,
    external_providers,
};

/// Builder-style DB fixtures for Hive, mirroring the structure used in `IAMRusty/tests/fixtures/db`.
pub struct DbFixtures;

impl DbFixtures {
    pub fn resource() -> ResourceFixtureBuilder {
        ResourceFixtureBuilder::new()
    }

    pub fn permission() -> PermissionFixtureBuilder {
        PermissionFixtureBuilder::new()
    }

    pub fn organization() -> OrganizationFixtureBuilder {
        OrganizationFixtureBuilder::new()
    }

    pub fn organization_member() -> OrganizationMemberFixtureBuilder {
        OrganizationMemberFixtureBuilder::new()
    }

    pub fn role_permission() -> RolePermissionFixtureBuilder {
        RolePermissionFixtureBuilder::new()
    }

    pub fn member_role_permission_link() -> MemberRolePermissionLinkBuilder {
        MemberRolePermissionLinkBuilder::new()
    }

    pub fn external_provider() -> ExternalProviderFixtureBuilder {
        ExternalProviderFixtureBuilder::new()
    }

    /// Convenience method to create minimal RBAC data for an organization and attach the owner as a member with all permissions.
    pub async fn create_org_with_owner(
        db: &DatabaseConnection,
        owner_user_id: Uuid,
    ) -> anyhow::Result<organizations::Model> {
        Self::create_org(db, owner_user_id, HashMap::from([(owner_user_id.to_string(), "owner".to_string())]))
            .await
    }

    
    /// Convenience method to create minimal RBAC data for an organization and attach the owner as a member with all permissions.
    pub async fn create_org(
        db: &DatabaseConnection,
        owner_user_id: Uuid,
        user_rights: HashMap<String, String>,
    ) -> anyhow::Result<organizations::Model> {

        // Create organization
        let org = Self::organization()
            .owner_user_id(owner_user_id)
            .name("Test Org")
            .slug(&format!("test-org-{}", &Uuid::new_v4().to_string()[..8]))
            .description(Some("Seeded org"))
            .commit(Arc::new(db.clone()))
            .await?;

        let mut members: Vec<organization_members::Model> = Vec::new();
        
        for (user_id, _perm) in &user_rights {
            let member = Self::organization_member()
                .organization_id(org.id)
                .user_id(user_id.parse::<Uuid>().unwrap())
                .status("active")
                .joined_now()
                .commit(Arc::new(db.clone()))
                .await?;
            members.push(member);
        }

         // Create role-permissions chain for owner on organization resource and link to member
        for perm in ["owner", "admin", "write", "read"] {
            let rp = Self::role_permission()
                .organization_id(org.id)
                .permission_id(perm)
                .resource_id("organization")
                .name(&format!("org_{}_role", perm))
                .description(Some(&format!("{} on organization", perm)))
                .commit(Arc::new(db.clone()))
                .await?;

            for member in &members {
                if user_rights.get(member.user_id.to_string().as_str()).unwrap() == perm {
                    let _ = Self::member_role_permission_link()
                        .member_id(member.id)
                        .role_permission_id(rp.id)
                        .commit(Arc::new(db.clone()))
                        .await?;
                }
            }
        }

        Ok(org)
    }
}

/// Backward-compat free function delegating to the new builder-style API.
pub async fn seed_org_with_owner(
    db: &DatabaseConnection,
    owner_user_id: Uuid,
) -> anyhow::Result<organizations::Model> {
    DbFixtures::create_org_with_owner(db, owner_user_id).await
}

// ========================= Builders & Fixtures =========================

pub struct ResourceFixtureBuilder {
    id: String,
    resource_type: String,
    name: String,
    description: Option<String>,
}

impl ResourceFixtureBuilder {
    pub fn new() -> Self {
        Self {
            id: "organization".to_string(),
            resource_type: "domain".to_string(),
            name: "organization".to_string(),
            description: Some("Organization resource".to_string()),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = resource_type.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(|d| d.into());
        self
    }

    pub async fn commit(self, db: Arc<DatabaseConnection>) -> anyhow::Result<resources::Model> {
        let model = resources::ActiveModel {
            id: Set(self.id),
            resource_type: Set(self.resource_type),
            name: Set(self.name),
            description: Set(self.description),
            created_at: Set(Utc::now()),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct PermissionFixtureBuilder {
    id: String,
    level: String,
    description: Option<String>,
}

impl PermissionFixtureBuilder {
    pub fn new() -> Self {
        Self {
            id: "read".to_string(),
            level: "read".to_string(),
            description: Some("read permission".to_string()),
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = level.into();
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(|d| d.into());
        self
    }

    pub async fn commit(self, db: Arc<DatabaseConnection>) -> anyhow::Result<permissions::Model> {
        let model = permissions::ActiveModel {
            id: Set(self.id),
            level: Set(self.level),
            description: Set(self.description),
            created_at: Set(Utc::now()),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct OrganizationFixtureBuilder {
    id: Uuid,
    name: String,
    slug: String,
    description: Option<String>,
    avatar_url: Option<String>,
    owner_user_id: Uuid,
    settings: serde_json::Value,
}

impl OrganizationFixtureBuilder {
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        let slug_suffix = &Uuid::new_v4().to_string()[..8];
        Self {
            id,
            name: "Test Org".to_string(),
            slug: format!("test-org-{}", slug_suffix),
            description: Some("Seeded org".to_string()),
            avatar_url: None,
            owner_user_id: Uuid::new_v4(),
            settings: serde_json::json!({}),
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = slug.into();
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(|d| d.into());
        self
    }

    pub fn avatar_url(mut self, avatar_url: Option<impl Into<String>>) -> Self {
        self.avatar_url = avatar_url.map(|a| a.into());
        self
    }

    pub fn owner_user_id(mut self, owner_user_id: Uuid) -> Self {
        self.owner_user_id = owner_user_id;
        self
    }

    pub fn settings(mut self, settings: serde_json::Value) -> Self {
        self.settings = settings;
        self
    }

    pub async fn commit(self, db: Arc<DatabaseConnection>) -> anyhow::Result<organizations::Model> {
        let now = Utc::now();
        let model = organizations::ActiveModel {
            id: Set(self.id),
            name: Set(self.name),
            slug: Set(self.slug),
            description: Set(self.description),
            avatar_url: Set(self.avatar_url),
            owner_user_id: Set(self.owner_user_id),
            settings: Set(self.settings),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct OrganizationMemberFixtureBuilder {
    id: Uuid,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
    status: String,
    invited_by_user_id: Option<Uuid>,
    invited_at: Option<chrono::DateTime<Utc>>,
    joined_at: Option<chrono::DateTime<Utc>>,
}

impl OrganizationMemberFixtureBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            organization_id: None,
            user_id: None,
            status: "active".to_string(),
            invited_by_user_id: None,
            invited_at: None,
            joined_at: None,
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn organization_id(mut self, organization_id: Uuid) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn invited_by_user_id(mut self, invited_by_user_id: Option<Uuid>) -> Self {
        self.invited_by_user_id = invited_by_user_id;
        self
    }

    pub fn invited_now(mut self) -> Self {
        self.invited_at = Some(Utc::now());
        self
    }

    pub fn joined_now(mut self) -> Self {
        self.joined_at = Some(Utc::now());
        self
    }

    pub async fn commit(
        self,
        db: Arc<DatabaseConnection>,
    ) -> anyhow::Result<organization_members::Model> {
        let now = Utc::now();
        let model = organization_members::ActiveModel {
            id: Set(self.id),
            organization_id: Set(self.organization_id.expect("organization_id is required")),
            user_id: Set(self.user_id.expect("user_id is required")),
            status: Set(self.status),
            invited_by_user_id: Set(self.invited_by_user_id),
            invited_at: Set(self.invited_at),
            joined_at: Set(self.joined_at),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct RolePermissionFixtureBuilder {
    id: Uuid,
    name: String,
    description: Option<String>,
    organization_id: Option<Uuid>,
    permission_id: Option<String>,
    resource_id: Option<String>,
}

impl RolePermissionFixtureBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "role".to_string(),
            description: None,
            organization_id: None,
            permission_id: None,
            resource_id: None,
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(|d| d.into());
        self
    }

    pub fn organization_id(mut self, organization_id: Uuid) -> Self {
        self.organization_id = Some(organization_id);
        self
    }

    pub fn permission_id(mut self, permission_id: impl Into<String>) -> Self {
        self.permission_id = Some(permission_id.into());
        self
    }

    pub fn resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    pub async fn commit(self, db: Arc<DatabaseConnection>) -> anyhow::Result<role_permissions::Model> {
        let model = role_permissions::ActiveModel {
            id: Set(self.id),
            name: Set(self.name),
            description: Set(self.description),
            organization_id: Set(self.organization_id.expect("organization_id is required")),
            permission_id: Set(self.permission_id.expect("permission_id is required")),
            resource_id: Set(self.resource_id.expect("resource_id is required")),
            created_at: Set(Utc::now()),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct MemberRolePermissionLinkBuilder {
    id: Uuid,
    member_id: Option<Uuid>,
    role_permission_id: Option<Uuid>,
}

impl MemberRolePermissionLinkBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            member_id: None,
            role_permission_id: None,
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn member_id(mut self, member_id: Uuid) -> Self {
        self.member_id = Some(member_id);
        self
    }

    pub fn role_permission_id(mut self, role_permission_id: Uuid) -> Self {
        self.role_permission_id = Some(role_permission_id);
        self
    }

    pub async fn commit(
        self,
        db: Arc<DatabaseConnection>,
    ) -> anyhow::Result<organization_member_role_permissions::Model> {
        let model = organization_member_role_permissions::ActiveModel {
            id: Set(self.id),
            member_id: Set(self.member_id.expect("member_id is required")),
            role_permission_id: Set(self.role_permission_id.expect("role_permission_id is required")),
            created_at: Set(Utc::now()),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}

pub struct ExternalProviderFixtureBuilder {
    id: Uuid,
    provider_source: String,
    name: String,
    config_schema: Option<serde_json::Value>,
    is_active: bool,
}

impl ExternalProviderFixtureBuilder {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            provider_source: "github".to_string(),
            name: "GitHub".to_string(),
            config_schema: None,
            is_active: true,
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn provider_source(mut self, source: impl Into<String>) -> Self {
        self.provider_source = source.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn config_schema(mut self, schema: Option<serde_json::Value>) -> Self {
        self.config_schema = schema;
        self
    }

    pub fn is_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    pub async fn commit(self, db: Arc<DatabaseConnection>) -> anyhow::Result<external_providers::Model> {
        let model = external_providers::ActiveModel {
            id: Set(self.id),
            provider_type: Set(self.provider_source),
            name: Set(self.name),
            config_schema: Set(self.config_schema),
            is_active: Set(self.is_active),
            created_at: Set(Utc::now()),
        }
        .insert(&*db)
        .await?;
        Ok(model)
    }
}
