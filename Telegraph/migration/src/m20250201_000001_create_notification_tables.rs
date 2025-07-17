use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create notifications table
        manager
            .create_table(
                Table::create()
                    .table(Notifications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Notifications::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Notifications::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notifications::Title)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notifications::Content)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Notifications::ContentType)
                            .string()
                            .not_null()
                            .default("application/json"),
                    )
                    .col(
                        ColumnDef::new(Notifications::IsRead)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Notifications::Priority)
                            .small_integer()
                            .not_null()
                            .default(3), // 1=high, 2=medium, 3=normal, 4=low
                    )
                    .col(
                        ColumnDef::new(Notifications::ExpiresAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Notifications::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Notifications::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Notifications::ReadAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create notification delivery table for tracking delivery attempts
        manager
            .create_table(
                Table::create()
                    .table(NotificationDeliveries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NotificationDeliveries::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::NotificationId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::DeliveryMethod)
                            .string()
                            .not_null(), // email, sms, push, in_app
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::Status)
                            .string()
                            .not_null()
                            .default("pending"), // pending, sent, delivered, failed, bounced
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::AttemptCount)
                            .small_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::LastAttemptAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::DeliveredAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::ErrorMessage)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(NotificationDeliveries::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notification_deliveries_notification_id")
                            .from(NotificationDeliveries::Table, NotificationDeliveries::NotificationId)
                            .to(Notifications::Table, Notifications::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for performance
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notifications_user_id")
                    .table(Notifications::Table)
                    .col(Notifications::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notifications_user_is_read")
                    .table(Notifications::Table)
                    .col(Notifications::UserId)
                    .col(Notifications::IsRead)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notifications_created_at")
                    .table(Notifications::Table)
                    .col(Notifications::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notifications_expires_at")
                    .table(Notifications::Table)
                    .col(Notifications::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notification_deliveries_notification_id")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::NotificationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_notification_deliveries_status")
                    .table(NotificationDeliveries::Table)
                    .col(NotificationDeliveries::Status)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order due to foreign key constraints
        manager
            .drop_table(Table::drop().table(NotificationDeliveries::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Notifications::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Notifications {
    Table,
    Id,
    UserId,
    Title,
    Content,
    ContentType,
    IsRead,
    Priority,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
    ReadAt,
}

#[derive(DeriveIden)]
pub enum NotificationDeliveries {
    Table,
    Id,
    NotificationId,
    DeliveryMethod,
    Status,
    AttemptCount,
    LastAttemptAt,
    DeliveredAt,
    ErrorMessage,
    CreatedAt,
    UpdatedAt,
} 