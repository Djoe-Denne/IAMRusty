//! Repository implementations using SeaORM for Hive domain

pub mod entity;

// Repository implementations
pub mod organization_repository;
pub mod organization_member_repository;
pub mod organization_role_repository;
pub mod organization_invitation_repository;
pub mod external_provider_repository;
pub mod external_link_repository;
pub mod sync_job_repository;
pub mod permission_repository;
pub mod resource_repository;
pub mod role_permission_repository;

// Re-export implementations for convenience
pub use organization_repository::OrganizationRepositoryImpl;
pub use organization_member_repository::OrganizationMemberRepositoryImpl;
pub use organization_role_repository::OrganizationRoleRepositoryImpl;
pub use organization_invitation_repository::OrganizationInvitationRepositoryImpl;
pub use external_provider_repository::ExternalProviderRepositoryImpl;
pub use external_link_repository::ExternalLinkRepositoryImpl;
pub use sync_job_repository::SyncJobRepositoryImpl;
pub use permission_repository::{PermissionRepository, PermissionRepositoryImpl};
pub use resource_repository::{ResourceRepository, ResourceRepositoryImpl};
pub use role_permission_repository::{RolePermissionRepository, RolePermissionRepositoryImpl}; 