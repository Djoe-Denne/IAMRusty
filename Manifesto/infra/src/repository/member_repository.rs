use async_trait::async_trait;
use manifesto_domain::entity::ProjectMember;
use manifesto_domain::port::{
    MemberReadRepository, MemberRepository, MemberWriteRepository,
    ProjectMemberRolePermissionReadRepository,
};
use manifesto_domain::value_objects::MemberSource;
use rustycog_core::error::DomainError;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter,
};
use std::sync::Arc;
use uuid::Uuid;

use super::entity::{prelude::ProjectMembers, project_members};
use super::project_member_role_permission_repository::ProjectMemberRolePermissionReadRepositoryImpl;

pub struct MemberMapper;

impl MemberMapper {
    pub fn to_domain(model: project_members::Model) -> Result<ProjectMember, DomainError> {
        Ok(ProjectMember {
            id: model.id,
            project_id: model.project_id,
            user_id: model.user_id,
            source: MemberSource::from_str(&model.source)?,
            added_by: model.added_by,
            added_at: model.added_at.naive_utc().and_utc(),
            removed_at: model.removed_at.map(|dt| dt.naive_utc().and_utc()),
            removal_reason: model.removal_reason,
            grace_period_ends_at: model
                .grace_period_ends_at
                .map(|dt| dt.naive_utc().and_utc()),
            last_access_at: model.last_access_at.map(|dt| dt.naive_utc().and_utc()),
            role_permissions: Vec::new(), // Will be loaded separately
        })
    }

    pub fn to_active_model(member: &ProjectMember) -> project_members::ActiveModel {
        project_members::ActiveModel {
            id: ActiveValue::Set(member.id),
            project_id: ActiveValue::Set(member.project_id),
            user_id: ActiveValue::Set(member.user_id),
            source: ActiveValue::Set(member.source.as_str().to_string()),
            added_by: ActiveValue::Set(member.added_by),
            added_at: ActiveValue::Set(member.added_at.into()),
            removed_at: ActiveValue::Set(member.removed_at.map(|dt| dt.into())),
            removal_reason: ActiveValue::Set(member.removal_reason.clone()),
            grace_period_ends_at: ActiveValue::Set(member.grace_period_ends_at.map(|dt| dt.into())),
            last_access_at: ActiveValue::Set(member.last_access_at.map(|dt| dt.into())),
        }
    }
}

#[derive(Clone)]
pub struct MemberReadRepositoryImpl {
    db: Arc<DatabaseConnection>,
    pmrp_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
}

impl MemberReadRepositoryImpl {
    pub fn new(
        db: Arc<DatabaseConnection>,
        pmrp_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
    ) -> Self {
        Self { db, pmrp_repo }
    }

    async fn load_with_permissions(
        &self,
        mut member: ProjectMember,
    ) -> Result<ProjectMember, DomainError> {
        let role_permissions = self.pmrp_repo.find_by_member(&member.id).await?;
        member.role_permissions = role_permissions;
        Ok(member)
    }
}

#[async_trait]
impl MemberReadRepository for MemberReadRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectMember>, DomainError> {
        let member = ProjectMembers::find_by_id(*id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match member {
            Some(model) => {
                let member = MemberMapper::to_domain(model)?;
                Ok(Some(self.load_with_permissions(member).await?))
            }
            None => Ok(None),
        }
    }

    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<ProjectMember>, DomainError> {
        let members = ProjectMembers::find()
            .filter(project_members::Column::ProjectId.eq(*project_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in members {
            let member = MemberMapper::to_domain(model)?;
            result.push(self.load_with_permissions(member).await?);
        }
        Ok(result)
    }

    async fn find_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<ProjectMember>, DomainError> {
        let member = ProjectMembers::find()
            .filter(project_members::Column::ProjectId.eq(*project_id))
            .filter(project_members::Column::UserId.eq(*user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        match member {
            Some(model) => {
                let member = MemberMapper::to_domain(model)?;
                Ok(Some(self.load_with_permissions(member).await?))
            }
            None => Ok(None),
        }
    }

    async fn list_with_filters(
        &self,
        project_id: &Uuid,
        source: Option<MemberSource>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError> {
        let query = ProjectMembers::find();
        let mut conditions =
            Condition::all().add(project_members::Column::ProjectId.eq(*project_id));

        if let Some(src) = source {
            conditions = conditions.add(project_members::Column::Source.eq(src.as_str()));
        }

        if active_only {
            conditions = conditions.add(project_members::Column::RemovedAt.is_null());
        }

        let members = query
            .filter(conditions)
            .paginate(self.db.as_ref(), page_size as u64)
            .fetch_page(page as u64)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        let mut result = Vec::new();
        for model in members {
            let member = MemberMapper::to_domain(model)?;
            result.push(self.load_with_permissions(member).await?);
        }
        Ok(result)
    }

    async fn count_active(&self, project_id: &Uuid) -> Result<i64, DomainError> {
        let count = ProjectMembers::find()
            .filter(project_members::Column::ProjectId.eq(*project_id))
            .filter(project_members::Column::RemovedAt.is_null())
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count as i64)
    }
}

#[derive(Clone)]
pub struct MemberWriteRepositoryImpl {
    db: Arc<DatabaseConnection>,
    pmrp_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
}

impl MemberWriteRepositoryImpl {
    pub fn new(
        db: Arc<DatabaseConnection>,
        pmrp_repo: Arc<ProjectMemberRolePermissionReadRepositoryImpl>,
    ) -> Self {
        Self { db, pmrp_repo }
    }

    async fn load_with_permissions(
        &self,
        mut member: ProjectMember,
    ) -> Result<ProjectMember, DomainError> {
        let role_permissions = self.pmrp_repo.find_by_member(&member.id).await?;
        member.role_permissions = role_permissions;
        Ok(member)
    }

    pub async fn save_with_connection<C>(
        db: &C,
        member: &ProjectMember,
    ) -> Result<ProjectMember, DomainError>
    where
        C: ConnectionTrait,
    {
        let exists = ProjectMembers::find_by_id(member.id)
            .one(db)
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?
            .is_some();

        if exists {
            let active_model = MemberMapper::to_active_model(member);
            let result = active_model
                .update(db)
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;
            MemberMapper::to_domain(result)
        } else {
            let active_model = MemberMapper::to_active_model(member);
            let inserted = active_model
                .insert(db)
                .await
                .map_err(|e| DomainError::internal_error(&e.to_string()))?;
            MemberMapper::to_domain(inserted)
        }
    }
}

#[async_trait]
impl MemberWriteRepository for MemberWriteRepositoryImpl {
    async fn save(&self, member: &ProjectMember) -> Result<ProjectMember, DomainError> {
        let member = Self::save_with_connection(self.db.as_ref(), member).await?;
        self.load_with_permissions(member).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        let result = ProjectMembers::delete_by_id(*id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(DomainError::entity_not_found(
                "ProjectMember",
                &id.to_string(),
            ));
        }

        Ok(())
    }

    async fn exists_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        let count = ProjectMembers::find()
            .filter(project_members::Column::ProjectId.eq(*project_id))
            .filter(project_members::Column::UserId.eq(*user_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| DomainError::internal_error(&e.to_string()))?;

        Ok(count > 0)
    }
}

#[derive(Clone)]
pub struct MemberRepositoryImpl {
    read_repo: Arc<dyn MemberReadRepository>,
    write_repo: Arc<dyn MemberWriteRepository>,
}

impl MemberRepositoryImpl {
    pub fn new(
        read_repo: Arc<dyn MemberReadRepository>,
        write_repo: Arc<dyn MemberWriteRepository>,
    ) -> Self {
        Self {
            read_repo,
            write_repo,
        }
    }
}

#[async_trait]
impl MemberReadRepository for MemberRepositoryImpl {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<ProjectMember>, DomainError> {
        self.read_repo.find_by_id(id).await
    }

    async fn find_by_project(&self, project_id: &Uuid) -> Result<Vec<ProjectMember>, DomainError> {
        self.read_repo.find_by_project(project_id).await
    }

    async fn find_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Option<ProjectMember>, DomainError> {
        self.read_repo
            .find_by_project_and_user(project_id, user_id)
            .await
    }

    async fn list_with_filters(
        &self,
        project_id: &Uuid,
        source: Option<MemberSource>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError> {
        self.read_repo
            .list_with_filters(project_id, source, active_only, page, page_size)
            .await
    }

    async fn count_active(&self, project_id: &Uuid) -> Result<i64, DomainError> {
        self.read_repo.count_active(project_id).await
    }
}

#[async_trait]
impl MemberWriteRepository for MemberRepositoryImpl {
    async fn save(&self, member: &ProjectMember) -> Result<ProjectMember, DomainError> {
        self.write_repo.save(member).await
    }

    async fn delete(&self, id: &Uuid) -> Result<(), DomainError> {
        self.write_repo.delete(id).await
    }

    async fn exists_by_project_and_user(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        self.write_repo
            .exists_by_project_and_user(project_id, user_id)
            .await
    }
}

impl MemberRepository for MemberRepositoryImpl {}
