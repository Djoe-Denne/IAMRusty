use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    dto::{
        CreateProjectRequest, PaginationRequest, ProjectDetailResponse, ProjectListResponse,
        ProjectResponse, UpdateProjectRequest,
    },
    usecase::ProjectUseCase,
    ApplicationError,
};
use manifesto_domain::value_objects::{OwnerType, ProjectStatus};

// =============================================================================
// Error Mapper
// =============================================================================

pub struct ProjectErrorMapper;

impl CommandErrorMapper for ProjectErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        if let Some(error) = error.downcast_ref::<ApplicationError>() {
            match error {
                ApplicationError::Domain(domain_error) => {
                    CommandError::business("domain_error", &domain_error.to_string())
                }
                ApplicationError::Validation(msg) => {
                    CommandError::validation("validation_failed", msg)
                }
                ApplicationError::NotFound(msg) => {
                    CommandError::business("not_found", msg)
                }
                ApplicationError::AlreadyExists(msg) => {
                    CommandError::business("already_exists", msg)
                }
                ApplicationError::Internal(msg) => {
                    CommandError::infrastructure("internal_error", msg)
                }
            }
        } else {
            CommandError::business("unknown_error", &error.to_string())
        }
    }
}

// =============================================================================
// Create Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct CreateProjectCommand {
    pub command_id: Uuid,
    pub request: CreateProjectRequest,
    pub user_id: Uuid,
}

impl CreateProjectCommand {
    pub fn new(request: CreateProjectRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for CreateProjectCommand {
    type Result = ProjectResponse;

    fn command_type(&self) -> &'static str {
        "create_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if self.request.name.trim().is_empty() {
            return Err(CommandError::validation(
                "empty_name",
                "Project name cannot be empty",
            ));
        }

        Ok(())
    }
}

pub struct CreateProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl CreateProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<CreateProjectCommand> for CreateProjectCommandHandler {
    async fn handle(&self, command: CreateProjectCommand) -> Result<ProjectResponse, CommandError> {
        self.project_usecase
            .create_project(&command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("create_failed", &e.to_string()))
    }
}

// =============================================================================
// Get Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GetProjectCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl GetProjectCommand {
    pub fn new(project_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetProjectCommand {
    type Result = ProjectResponse;

    fn command_type(&self) -> &'static str {
        "get_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct GetProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl GetProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetProjectCommand> for GetProjectCommandHandler {
    async fn handle(&self, command: GetProjectCommand) -> Result<ProjectResponse, CommandError> {
        self.project_usecase
            .get_project(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("get_failed", &e.to_string()))
    }
}

// =============================================================================
// Get Project Detail Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct GetProjectDetailCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Option<Uuid>,
}

impl GetProjectDetailCommand {
    pub fn new(project_id: Uuid, user_id: Option<Uuid>) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetProjectDetailCommand {
    type Result = ProjectDetailResponse;

    fn command_type(&self) -> &'static str {
        "get_project_detail"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct GetProjectDetailCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl GetProjectDetailCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<GetProjectDetailCommand> for GetProjectDetailCommandHandler {
    async fn handle(
        &self,
        command: GetProjectDetailCommand,
    ) -> Result<ProjectDetailResponse, CommandError> {
        self.project_usecase
            .get_project_detail(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("get_detail_failed", &e.to_string()))
    }
}

// =============================================================================
// Update Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct UpdateProjectCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub request: UpdateProjectRequest,
    pub user_id: Uuid,
}

impl UpdateProjectCommand {
    pub fn new(project_id: Uuid, request: UpdateProjectRequest, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            request,
            user_id,
        }
    }
}

#[async_trait]
impl Command for UpdateProjectCommand {
    type Result = ProjectResponse;

    fn command_type(&self) -> &'static str {
        "update_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        if let Some(name) = &self.request.name {
            if name.trim().is_empty() {
                return Err(CommandError::validation(
                    "empty_name",
                    "Project name cannot be empty",
                ));
            }
        }
        Ok(())
    }
}

pub struct UpdateProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl UpdateProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<UpdateProjectCommand> for UpdateProjectCommandHandler {
    async fn handle(&self, command: UpdateProjectCommand) -> Result<ProjectResponse, CommandError> {
        self.project_usecase
            .update_project(command.project_id, &command.request, command.user_id)
            .await
            .map_err(|e| CommandError::business("update_failed", &e.to_string()))
    }
}

// =============================================================================
// Delete Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct DeleteProjectCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
}

impl DeleteProjectCommand {
    pub fn new(project_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for DeleteProjectCommand {
    type Result = ();

    fn command_type(&self) -> &'static str {
        "delete_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct DeleteProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl DeleteProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<DeleteProjectCommand> for DeleteProjectCommandHandler {
    async fn handle(&self, command: DeleteProjectCommand) -> Result<(), CommandError> {
        self.project_usecase
            .delete_project(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("delete_failed", &e.to_string()))
    }
}

// =============================================================================
// List Projects Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct ListProjectsCommand {
    pub command_id: Uuid,
    pub owner_type: Option<String>,
    pub owner_id: Option<Uuid>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub pagination: PaginationRequest,
}

impl ListProjectsCommand {
    pub fn new(
        owner_type: Option<String>,
        owner_id: Option<Uuid>,
        status: Option<String>,
        search: Option<String>,
        pagination: PaginationRequest,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            owner_type,
            owner_id,
            status,
            search,
            pagination,
        }
    }
}

#[async_trait]
impl Command for ListProjectsCommand {
    type Result = ProjectListResponse;

    fn command_type(&self) -> &'static str {
        "list_projects"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ListProjectsCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl ListProjectsCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<ListProjectsCommand> for ListProjectsCommandHandler {
    async fn handle(
        &self,
        command: ListProjectsCommand,
    ) -> Result<ProjectListResponse, CommandError> {
        let owner_type = command
            .owner_type
            .as_ref()
            .map(|s| OwnerType::from_str(s))
            .transpose()
            .map_err(|e| CommandError::validation("invalid_owner_type", &e.to_string()))?;

        let status = command
            .status
            .as_ref()
            .map(|s| ProjectStatus::from_str(s))
            .transpose()
            .map_err(|e| CommandError::validation("invalid_status", &e.to_string()))?;

        self.project_usecase
            .list_projects(
                owner_type,
                command.owner_id,
                status,
                command.search,
                &command.pagination,
            )
            .await
            .map_err(|e| CommandError::business("list_failed", &e.to_string()))
    }
}

// =============================================================================
// Publish Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct PublishProjectCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
}

impl PublishProjectCommand {
    pub fn new(project_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for PublishProjectCommand {
    type Result = ProjectResponse;

    fn command_type(&self) -> &'static str {
        "publish_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct PublishProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl PublishProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<PublishProjectCommand> for PublishProjectCommandHandler {
    async fn handle(&self, command: PublishProjectCommand) -> Result<ProjectResponse, CommandError> {
        self.project_usecase
            .publish_project(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("publish_failed", &e.to_string()))
    }
}

// =============================================================================
// Archive Project Command
// =============================================================================

#[derive(Debug, Clone)]
pub struct ArchiveProjectCommand {
    pub command_id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
}

impl ArchiveProjectCommand {
    pub fn new(project_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for ArchiveProjectCommand {
    type Result = ProjectResponse;

    fn command_type(&self) -> &'static str {
        "archive_project"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Ok(())
    }
}

pub struct ArchiveProjectCommandHandler {
    project_usecase: Arc<dyn ProjectUseCase>,
}

impl ArchiveProjectCommandHandler {
    pub fn new(project_usecase: Arc<dyn ProjectUseCase>) -> Self {
        Self { project_usecase }
    }
}

#[async_trait]
impl CommandHandler<ArchiveProjectCommand> for ArchiveProjectCommandHandler {
    async fn handle(&self, command: ArchiveProjectCommand) -> Result<ProjectResponse, CommandError> {
        self.project_usecase
            .archive_project(command.project_id, command.user_id)
            .await
            .map_err(|e| CommandError::business("archive_failed", &e.to_string()))
    }
}
