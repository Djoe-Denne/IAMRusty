//! Empty Lazaret migrator (no tables this slice).

pub use sea_orm_migration::prelude::*;

mod m20260915_000001_apparatus_kv_entries;

/// Lazaret schema migrator.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260915_000001_apparatus_kv_entries::Migration)]
    }
}
