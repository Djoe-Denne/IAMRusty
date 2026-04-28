use async_trait::async_trait;
use rustycog_core::error::DomainError;
use std::sync::Arc;
use uuid::Uuid;

use crate::entity::ProjectMember;
use crate::port::MemberRepository;
use crate::value_objects::MemberSource;

#[async_trait]
pub trait MemberService: Send + Sync {
    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectMember, DomainError>;

    async fn add_member(&self, member: ProjectMember) -> Result<ProjectMember, DomainError>;

    async fn update_member(&self, member: ProjectMember) -> Result<ProjectMember, DomainError>;

    async fn remove_member(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
        grace_period_days: Option<i64>,
    ) -> Result<(), DomainError>;

    async fn list_members(
        &self,
        project_id: &Uuid,
        source: Option<MemberSource>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError>;

    async fn count_active_members(&self, project_id: &Uuid) -> Result<i64, DomainError>;

    async fn check_member_exists(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError>;
}

pub struct MemberServiceImpl<MR>
where
    MR: MemberRepository,
{
    member_repo: Arc<MR>,
}

impl<MR> MemberServiceImpl<MR>
where
    MR: MemberRepository,
{
    pub fn new(member_repo: Arc<MR>) -> Self {
        Self { member_repo }
    }
}

#[async_trait]
impl<MR> MemberService for MemberServiceImpl<MR>
where
    MR: MemberRepository,
{
    async fn get_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<ProjectMember, DomainError> {
        self.member_repo
            .find_by_project_and_user(&project_id, &user_id)
            .await?
            .ok_or_else(|| {
                DomainError::entity_not_found(
                    "ProjectMember",
                    &format!("{}/{}", project_id, user_id),
                )
            })
    }

    async fn add_member(&self, member: ProjectMember) -> Result<ProjectMember, DomainError> {
        // Validate member
        member.validate()?;

        // Check if member already exists
        if self
            .check_member_exists(&member.project_id, &member.user_id)
            .await?
        {
            return Err(DomainError::resource_already_exists(
                "ProjectMember",
                &member.user_id.to_string(),
            ));
        }

        // Save member
        self.member_repo.save(&member).await
    }

    async fn update_member(&self, member: ProjectMember) -> Result<ProjectMember, DomainError> {
        // Validate member
        member.validate()?;

        // Save member
        self.member_repo.save(&member).await
    }

    async fn remove_member(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
        grace_period_days: Option<i64>,
    ) -> Result<(), DomainError> {
        let mut member = self.get_member(*project_id, *user_id).await?;
        member.remove(None, grace_period_days);
        self.member_repo.save(&member).await?;
        Ok(())
    }

    async fn list_members(
        &self,
        project_id: &Uuid,
        source: Option<MemberSource>,
        active_only: bool,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<ProjectMember>, DomainError> {
        self.member_repo
            .list_with_filters(project_id, source, active_only, page, page_size)
            .await
    }

    async fn count_active_members(&self, project_id: &Uuid) -> Result<i64, DomainError> {
        self.member_repo.count_active(project_id).await
    }

    async fn check_member_exists(
        &self,
        project_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<bool, DomainError> {
        self.member_repo
            .exists_by_project_and_user(project_id, user_id)
            .await
    }
}
