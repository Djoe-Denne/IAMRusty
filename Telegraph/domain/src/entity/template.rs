//! Message template domain entities for Telegraph communication service

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::DomainError;
use super::communication::CommunicationMode;

/// Message template for generating standardized messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    /// Template ID
    pub id: Uuid,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Communication mode this template is for
    pub mode: CommunicationMode,
    /// Template content with placeholders
    pub content: TemplateContent,
    /// Default variables for the template
    pub default_variables: HashMap<String, String>,
    /// When the template was created
    pub created_at: DateTime<Utc>,
    /// When the template was last updated
    pub updated_at: DateTime<Utc>,
    /// Whether the template is active
    pub active: bool,
}

/// Template content based on communication mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TemplateContent {
    /// Email template content
    Email {
        /// Email subject template
        subject: String,
        /// HTML body template
        html_body: Option<String>,
        /// Plain text body template
        text_body: String,
    },
    /// Push notification template content
    Notification {
        /// Notification title template
        title: String,
        /// Notification body template
        body: String,
        /// Default data payload
        default_data: HashMap<String, String>,
    },
    /// SMS template content
    Sms {
        /// SMS message template
        text: String,
    },
}

impl MessageTemplate {
    /// Create a new message template
    pub fn new(
        name: String,
        mode: CommunicationMode,
        content: TemplateContent,
    ) -> Result<Self, DomainError> {
        // Validate that the mode matches the content
        Self::validate_mode_content_match(&mode, &content)?;
        
        let now = Utc::now();
        
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            description: None,
            mode,
            content,
            default_variables: HashMap::new(),
            created_at: now,
            updated_at: now,
            active: true,
        })
    }
    
    /// Set template description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Add default variable
    pub fn with_default_variable(mut self, key: String, value: String) -> Self {
        self.default_variables.insert(key, value);
        self
    }
    
    /// Set template as inactive
    pub fn deactivate(&mut self) {
        self.active = false;
        self.updated_at = Utc::now();
    }
    
    /// Set template as active
    pub fn activate(&mut self) {
        self.active = true;
        self.updated_at = Utc::now();
    }
    
    /// Render template with provided variables
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<RenderedTemplate, DomainError> {
        if !self.active {
            return Err(DomainError::template_not_found(format!("Template '{}' is inactive", self.name)));
        }
        
        // Merge default variables with provided variables (provided variables take precedence)
        let mut merged_variables = self.default_variables.clone();
        for (key, value) in variables {
            merged_variables.insert(key.clone(), value.clone());
        }
        
        match &self.content {
            TemplateContent::Email { subject, html_body, text_body } => {
                Ok(RenderedTemplate::Email {
                    subject: Self::replace_placeholders(subject, &merged_variables)?,
                    html_body: html_body.as_ref().map(|body| Self::replace_placeholders(body, &merged_variables)).transpose()?,
                    text_body: Self::replace_placeholders(text_body, &merged_variables)?,
                })
            }
            TemplateContent::Notification { title, body, default_data } => {
                let mut data = default_data.clone();
                // Add variables to data payload
                for (key, value) in &merged_variables {
                    data.insert(format!("var_{}", key), value.clone());
                }
                
                Ok(RenderedTemplate::Notification {
                    title: Self::replace_placeholders(title, &merged_variables)?,
                    body: Self::replace_placeholders(body, &merged_variables)?,
                    data,
                })
            }
            TemplateContent::Sms { text } => {
                Ok(RenderedTemplate::Sms {
                    text: Self::replace_placeholders(text, &merged_variables)?,
                })
            }
        }
    }
    
    /// Replace placeholders in text with variables
    fn replace_placeholders(text: &str, variables: &HashMap<String, String>) -> Result<String, DomainError> {
        let mut result = text.to_string();
        
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        // Check for unreplaced placeholders
        if result.contains("{{") && result.contains("}}") {
            return Err(DomainError::invalid_message("Template contains unreplaced placeholders".to_string()));
        }
        
        Ok(result)
    }
    
    /// Validate that the mode matches the content
    fn validate_mode_content_match(mode: &CommunicationMode, content: &TemplateContent) -> Result<(), DomainError> {
        match (mode, content) {
            (CommunicationMode::Email, TemplateContent::Email { .. }) => Ok(()),
            (CommunicationMode::Notification, TemplateContent::Notification { .. }) => Ok(()),
            _ => Err(DomainError::invalid_message(
                format!("Communication mode {:?} does not match template content type", mode)
            )),
        }
    }
}

/// Rendered template ready for message creation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RenderedTemplate {
    /// Rendered email template
    Email {
        /// Rendered subject
        subject: String,
        /// Rendered HTML body
        html_body: Option<String>,
        /// Rendered text body
        text_body: String,
    },
    /// Rendered notification template
    Notification {
        /// Rendered title
        title: String,
        /// Rendered body
        body: String,
        /// Data payload
        data: HashMap<String, String>,
    },
    /// Rendered SMS template
    Sms {
        /// Rendered text
        text: String,
    },
} 