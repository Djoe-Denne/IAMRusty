//! Lazaret schema migrator (`apparatus_kv_entries`, `apparatus_enrollments`).

pub use sea_orm_migration::prelude::*;

mod m20260915_000001_apparatus_kv_entries;
mod m20260917_000002_apparatus_enrollments;

/// Lazaret schema migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260915_000001_apparatus_kv_entries::Migration),
            Box::new(m20260917_000002_apparatus_enrollments::Migration),
        ]
    }
}
