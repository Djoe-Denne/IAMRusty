//! Privileged binding grant snapshot reader (not world-read `AuthZ`).

use async_trait::async_trait;
use uuid::Uuid;

use crate::dto::{BindingGrantSnapshotResponse, UpsertBindingConsentRequest};
use crate::ApplicationError;

/// Loads binding, consents, and optional principal membership from storage.
#[async_trait]
pub trait BindingGrantSnapshotReader: Send + Sync {
    /// Load the grant snapshot for a binding.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::NotFound`] when the binding is missing,
    /// or [`ApplicationError::Internal`] when the read fails.
    async fn load(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        principal: Option<Uuid>,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError>;
}

/// Upserts a consent row and bumps the binding `grant_revision` in one transaction.
#[async_trait]
pub trait BindingConsentWriter: Send + Sync {
    /// Persist consent status and return the updated snapshot (no principal).
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::NotFound`] when the binding is missing,
    /// [`ApplicationError::Validation`] when capability or status is invalid,
    /// or [`ApplicationError::Internal`] when the write fails.
    async fn upsert(
        &self,
        project_id: Uuid,
        component_id: Uuid,
        request: &UpsertBindingConsentRequest,
    ) -> Result<BindingGrantSnapshotResponse, ApplicationError>;
}
