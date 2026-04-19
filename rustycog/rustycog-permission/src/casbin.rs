//! Casbin-based permission engine implementation
//!
//! This module provides a Casbin-based implementation of the PermissionEngine trait,
//! allowing for hierarchical role-based access control with policy files.

use std::sync::Arc;

use casbin::{DefaultModel, Enforcer, MemoryAdapter, CoreApi, MgmtApi, EnforceContext};
use rustycog_core::error::DomainError;
use tracing::{debug, error};
use async_trait::async_trait;
use uuid::Uuid;

use crate::{PermissionEngine, Permission, PermissionsFetcher, ResourceId};

const ANONYMOUS_SUBJECT: &str = "anonymous";

/// Casbin-based permission engine implementation
/// 
/// This engine uses Casbin for policy-based access control with support for
/// hierarchical role relationships defined in policy files.
pub struct CasbinPermissionEngine {
    model_path: String,   // Path to model.conf
    permissions_fetch: Arc<dyn PermissionsFetcher>,
}

impl CasbinPermissionEngine {
    /// Create a new Casbin permission engine
    /// 
    /// # Arguments
    /// * `model_path` - Path to the Casbin model configuration file
    /// * `permissions_fetch` - Provider used to fetch user permissions for resources
    pub async fn new(model_path: String, permissions_fetch: Arc<dyn PermissionsFetcher>) -> Result<Self, DomainError> {
        // Defer model validation to enforcer creation time
        Ok(Self { model_path, permissions_fetch })
    }
    
    /// Create a new enforcer instance with the configured model and policy
    async fn create_enforcer(&self) -> Result<Enforcer, DomainError> {
        // Create model from file
        let model = DefaultModel::from_file(&self.model_path)
            .await
            .map_err(|e| DomainError::Internal {
                message: format!("Failed to load Casbin model from {}: {}", self.model_path, e),
            })?;

        // Use in-memory adapter for per-request transient policies
        let adapter = MemoryAdapter::default();

        // Create enforcer with model and adapter
        let mut enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| DomainError::Internal {
                message: format!("Failed to create Casbin enforcer: {}", e),
            })?;

        // No named grouping policies needed; the engine injects derived policies directly

        Ok(enforcer)
    }
}

#[async_trait]
impl PermissionEngine for CasbinPermissionEngine {
    /// Check if user has the target permission based on their current roles
    async fn has_permission(
        &self,
        user_id: Option<Uuid>,
        resource_ids: Vec<ResourceId>,
        target_permission: Permission,
        _settings: serde_json::Value,
    ) -> Result<bool, DomainError> {
        debug!("CasbinPermissionEngine::has_permission: user_id: {:?}, resource_ids: {:?}, target_permission: {:?}", user_id, resource_ids, target_permission);
        let mut enforcer = self.create_enforcer().await?;

        // Fetch user permissions for these resources
        let permissions = self
            .permissions_fetch
            .fetch_permissions(user_id, resource_ids.clone())
            .await?;

        debug!("CasbinPermissionEngine::has_permission: permissions: {:?}", permissions);
        let subject = user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| ANONYMOUS_SUBJECT.to_string());
        let mut policy_vec = vec![subject];

        
        for index in 0..resource_ids.len() {
            policy_vec.push(resource_ids[index].id().to_string());
            let policy_name = if index != 0 { format!("p{}", index+1) } else { "p".to_string() };
            
            // Add policies (subject=user_id, object=resource_key, action=permission)
            for permission in &permissions {
                // Expand hierarchical permissions: owner > admin > write > read
                let implied: &[&str] = match permission {
                    Permission::Owner => &["owner", "admin", "write", "read"],
                    Permission::Admin => &["admin", "write", "read"],
                    Permission::Write => &["write", "read"],
                    Permission::Read => &["read"],
                };
                for action in implied {
                    let mut policy_vec = policy_vec.clone();
                    policy_vec.push((*action).to_string());
                    debug!("CasbinPermissionEngine::has_permission: adding policy: {:?} for policy name: {}", policy_vec, policy_name);
                    let _ = enforcer
                        .add_named_policy(policy_name.as_str(), policy_vec.clone())
                        .await
                        .map_err(|e| {
                            error!("Failed to add policy: {}", e);
                            DomainError::Internal {
                            message: format!("Failed to add policy: {}", e),
                        }
                    })?;
                
                }
            }
        }


        policy_vec.push(target_permission.as_str().to_string());
        debug!("CasbinPermissionEngine::has_permission: policy_vec: {:?}", policy_vec);

        let decision = if resource_ids.len() > 1 { 
            let ctx = EnforceContext::new(resource_ids.len().to_string().as_str());
            enforcer
                .enforce_with_context(ctx, policy_vec)
                .map_err(|e| {
                    error!("Failed to enforce permission check: {}", e);
                    DomainError::Internal {
                    message: format!("Failed to enforce permission check: {}", e),
                }
            })?
        } else { 
            enforcer
                .enforce(policy_vec)
                .map_err(|e| {
                    error!("Failed to enforce permission check: {}", e);
                    DomainError::Internal {
                    message: format!("Failed to enforce permission check: {}", e),
                }
            })?
        };

        debug!("CasbinPermissionEngine::has_permission: decision: {:?}", decision);

        Ok(decision)
    }
}