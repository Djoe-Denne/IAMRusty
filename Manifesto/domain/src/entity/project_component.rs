use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::value_objects::ComponentStatus;
use rustycog_core::error::DomainError;

#[derive(Debug, Clone)]
pub struct ProjectComponent {
    pub id: Uuid,
    pub project_id: Uuid,
    pub component_type: String,
    pub status: ComponentStatus,
    pub added_at: DateTime<Utc>,
    pub configured_at: Option<DateTime<Utc>>,
    pub activated_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
}

impl ProjectComponent {
    /// Create a new project component
    pub fn new(project_id: Uuid, component_type: String) -> Result<Self, DomainError> {
        if component_type.trim().is_empty() {
            return Err(DomainError::invalid_input(
                "Component type cannot be empty",
            ));
        }

        if component_type.len() > 100 {
            return Err(DomainError::invalid_input(
                "Component type cannot exceed 100 characters",
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            project_id,
            component_type,
            status: ComponentStatus::Pending,
            added_at: Utc::now(),
            configured_at: None,
            activated_at: None,
            disabled_at: None,
        })
    }

    /// Transition the component to a new status
    pub fn transition_status(&mut self, new_status: ComponentStatus) -> Result<(), DomainError> {
        let old_status = self.status;
        let transitioned = old_status.transition_to(new_status)?;
        self.status = transitioned;

        // Update timestamps based on status
        let now = Utc::now();
        match new_status {
            ComponentStatus::Configured => {
                if self.configured_at.is_none() {
                    self.configured_at = Some(now);
                }
            }
            ComponentStatus::Active => {
                if self.activated_at.is_none() {
                    self.activated_at = Some(now);
                }
            }
            ComponentStatus::Disabled => {
                if self.disabled_at.is_none() {
                    self.disabled_at = Some(now);
                }
            }
            ComponentStatus::Pending => {}
        }

        Ok(())
    }

    /// Check if the component is active
    pub fn is_active(&self) -> bool {
        self.status == ComponentStatus::Active
    }

    /// Validate the component
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.component_type.trim().is_empty() {
            return Err(DomainError::invalid_input(
                "Component type cannot be empty",
            ));
        }

        if self.component_type.len() > 100 {
            return Err(DomainError::invalid_input(
                "Component type cannot exceed 100 characters",
            ));
        }

        Ok(())
    }
}

