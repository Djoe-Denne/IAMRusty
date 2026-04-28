// Common fixtures are now in rustycog-testing
// pub mod common;
pub mod db;
pub mod github;
pub mod gitlab;

#[allow(unused_imports)]
pub use db::DbFixtures;
#[allow(unused_imports)]
pub use github::GitHubFixtures;
#[allow(unused_imports)]
pub use gitlab::GitLabFixtures;
