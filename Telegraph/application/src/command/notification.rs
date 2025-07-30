use crate::usecase::{
    GetNotificationsInput, GetNotificationsResponse, GetUnreadCountInput, GetUnreadCountResponse,
    MarkNotificationReadInput, MarkNotificationReadResponse, NotificationUseCaseError,
    NotificationUseCaseTrait,
};
use async_trait::async_trait;
use rustycog_command::{Command, CommandError, CommandErrorMapper, CommandHandler};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Command to get user notifications with pagination and filtering
#[derive(Debug, Clone, Validate)]
pub struct GetNotificationsCommand {
    pub command_id: Uuid,
    pub user_id: Uuid,
    #[validate(range(min = 0, max = 100))]
    pub page: Option<u8>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u8>,
    pub unread_only: Option<bool>,
}

impl GetNotificationsCommand {
    pub fn new(
        user_id: Uuid,
        page: Option<u8>,
        per_page: Option<u8>,
        unread_only: Option<bool>,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
            page,
            per_page,
            unread_only,
        }
    }
}

#[async_trait]
impl Command for GetNotificationsCommand {
    type Result = GetNotificationsResponse;

    fn command_type(&self) -> &'static str {
        "get_notifications"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        Validate::validate(self).map_err(|e| {
            CommandError::validation("VALIDATION_ERROR", format!("Validation failed: {}", e))
        })
    }
}

/// Handler for GetNotificationsCommand
pub struct GetNotificationsCommandHandler {
    notification_usecase: Arc<dyn NotificationUseCaseTrait>,
}

impl GetNotificationsCommandHandler {
    pub fn new(notification_usecase: Arc<dyn NotificationUseCaseTrait>) -> Self {
        Self {
            notification_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<GetNotificationsCommand> for GetNotificationsCommandHandler {
    async fn handle(
        &self,
        command: GetNotificationsCommand,
    ) -> Result<GetNotificationsResponse, CommandError> {
        let input = GetNotificationsInput {
            user_id: command.user_id,
            page: command.page,
            per_page: command.per_page,
            unread_only: command.unread_only,
        };

        self.notification_usecase
            .get_notifications(input)
            .await
            .map_err(|e| match e {
                NotificationUseCaseError::ValidationError(msg) => {
                    CommandError::validation("VALIDATION_ERROR", msg)
                }
                NotificationUseCaseError::Domain(domain_error) => {
                    CommandError::business("DOMAIN_ERROR", domain_error.to_string())
                }
            })
    }
}

/// Error mapper for GetNotificationsCommand
pub struct GetNotificationsErrorMapper;

impl CommandErrorMapper for GetNotificationsErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        CommandError::infrastructure("INFRASTRUCTURE_ERROR", error.to_string())
    }
}

/// Command to get unread notification count
#[derive(Debug, Clone)]
pub struct GetUnreadCountCommand {
    pub command_id: Uuid,
    pub user_id: Uuid,
}

impl GetUnreadCountCommand {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            user_id,
        }
    }
}

#[async_trait]
impl Command for GetUnreadCountCommand {
    type Result = GetUnreadCountResponse;

    fn command_type(&self) -> &'static str {
        "get_unread_count"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // No complex validation needed for this command
        Ok(())
    }
}

/// Handler for GetUnreadCountCommand
pub struct GetUnreadCountCommandHandler {
    notification_usecase: Arc<dyn NotificationUseCaseTrait>,
}

impl GetUnreadCountCommandHandler {
    pub fn new(notification_usecase: Arc<dyn NotificationUseCaseTrait>) -> Self {
        Self {
            notification_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<GetUnreadCountCommand> for GetUnreadCountCommandHandler {
    async fn handle(
        &self,
        command: GetUnreadCountCommand,
    ) -> Result<GetUnreadCountResponse, CommandError> {
        let input = GetUnreadCountInput {
            user_id: command.user_id,
        };

        self.notification_usecase
            .get_unread_count(input)
            .await
            .map_err(|e| match e {
                NotificationUseCaseError::ValidationError(msg) => {
                    CommandError::validation("VALIDATION_ERROR", msg)
                }
                NotificationUseCaseError::Domain(domain_error) => {
                    CommandError::business("DOMAIN_ERROR", domain_error.to_string())
                }
            })
    }
}

/// Error mapper for GetUnreadCountCommand
pub struct GetUnreadCountErrorMapper;

impl CommandErrorMapper for GetUnreadCountErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        CommandError::infrastructure("INFRASTRUCTURE_ERROR", error.to_string())
    }
}

/// Command to mark a notification as read
#[derive(Debug, Clone)]
pub struct MarkNotificationReadCommand {
    pub command_id: Uuid,
    pub notification_id: Uuid,
    pub user_id: Uuid,
}

impl MarkNotificationReadCommand {
    pub fn new(notification_id: Uuid, user_id: Uuid) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            notification_id,
            user_id,
        }
    }
}

#[async_trait]
impl Command for MarkNotificationReadCommand {
    type Result = MarkNotificationReadResponse;

    fn command_type(&self) -> &'static str {
        "mark_notification_read"
    }

    fn command_id(&self) -> Uuid {
        self.command_id
    }

    fn validate(&self) -> Result<(), CommandError> {
        // Basic validation that IDs are not nil
        if self.notification_id == Uuid::nil() {
            return Err(CommandError::validation(
                "INVALID_NOTIFICATION_ID",
                "Notification ID cannot be nil",
            ));
        }
        if self.user_id == Uuid::nil() {
            return Err(CommandError::validation(
                "INVALID_USER_ID",
                "User ID cannot be nil",
            ));
        }
        Ok(())
    }
}

/// Handler for MarkNotificationReadCommand
pub struct MarkNotificationReadCommandHandler {
    notification_usecase: Arc<dyn NotificationUseCaseTrait>,
}

impl MarkNotificationReadCommandHandler {
    pub fn new(notification_usecase: Arc<dyn NotificationUseCaseTrait>) -> Self {
        Self {
            notification_usecase,
        }
    }
}

#[async_trait]
impl CommandHandler<MarkNotificationReadCommand> for MarkNotificationReadCommandHandler {
    async fn handle(
        &self,
        command: MarkNotificationReadCommand,
    ) -> Result<MarkNotificationReadResponse, CommandError> {
        let input = MarkNotificationReadInput {
            notification_id: command.notification_id,
            user_id: command.user_id,
        };

        self.notification_usecase
            .mark_notification_read(input)
            .await
            .map_err(|e| match e {
                NotificationUseCaseError::ValidationError(msg) => {
                    CommandError::validation("VALIDATION_ERROR", msg)
                }
                NotificationUseCaseError::Domain(domain_error) => {
                    CommandError::business("DOMAIN_ERROR", domain_error.to_string())
                }
            })
    }
}

/// Error mapper for MarkNotificationReadCommand
pub struct MarkNotificationReadErrorMapper;

impl CommandErrorMapper for MarkNotificationReadErrorMapper {
    fn map_error(&self, error: Box<dyn std::error::Error + Send + Sync>) -> CommandError {
        CommandError::infrastructure("INFRASTRUCTURE_ERROR", error.to_string())
    }
}
