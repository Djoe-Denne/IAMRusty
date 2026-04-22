//! Test fixtures for Manifesto
//!
//! Provides builders for creating test data in the database

// Component-catalog wiremock fixtures live under `fixtures/component_service/`
// but are re-exported from `common` (see `Manifesto/tests/common.rs`) to
// avoid double-including the same file from two distinct module paths,
// which would yield two unrelated copies of `ComponentServiceMockService`.
pub mod db;

pub use db::DbFixtures;


