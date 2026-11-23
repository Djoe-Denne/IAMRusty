pub use sea_orm_migration::prelude::*;

mod m20220101_000001_initial_schema;
mod m20261108_000001_oauth_authentication_nonce;
mod m20261123_000008_profile_email_verifications;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_initial_schema::Migration),
            Box::new(m20261108_000001_oauth_authentication_nonce::Migration),
            Box::new(m20261123_000008_profile_email_verifications::Migration),
        ]
    }
}
