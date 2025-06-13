pub use sea_orm_migration::prelude::*;

mod m20220101_000001_create_table;
mod m20250524_074801_allow_multiple_providers_per_user;
mod m20250524_080442_support_multiple_emails_per_user;
mod m20250120_000001_add_password_and_email_verification;
mod m20250130_000001_make_username_optional;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20250524_074801_allow_multiple_providers_per_user::Migration),
            Box::new(m20250524_080442_support_multiple_emails_per_user::Migration),
            Box::new(m20250120_000001_add_password_and_email_verification::Migration),
            Box::new(m20250130_000001_make_username_optional::Migration),
        ]
    }
}
