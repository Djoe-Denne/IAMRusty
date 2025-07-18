//! Tera-based file template service implementation

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tera::{Tera, Context};
use tokio::sync::RwLock;
use uuid::Uuid;

use tracing::{info, warn, error, debug};

use telegraph_domain::{
    DomainError, TemplateService, MessageTemplate, RenderedTemplate, 
    CommunicationMode, TemplateContent
};
use telegraph_configuration::TemplateConfig;

/// File-based template service using Tera template engine
pub struct TeraTemplateService {
    /// Tera template engine instance
    tera: Arc<RwLock<Tera>>,
    /// Template configuration
    config: TemplateConfig,
    /// Template directory path
    template_dir: PathBuf,
}

impl TeraTemplateService {
    /// Create a new Tera template service
    pub fn new(config: TemplateConfig) -> Result<Self, DomainError> {
        let template_dir = PathBuf::from(&config.template_dir);
        
        // Create the template directory if it doesn't exist
        if !template_dir.exists() {
            std::fs::create_dir_all(&template_dir)
                .map_err(|e| DomainError::template_load_error(
                    format!("Failed to create template directory '{}': {}", config.template_dir, e)
                ))?;
            info!(
                directory = %config.template_dir,
                "Created template directory"
            );
        }
        
        // Initialize Tera with the template directory
        let tera = Self::initialize_tera(&template_dir)?;
        
        info!(
            directory = %config.template_dir,
            template_count = tera.get_template_names().count(),
            "Initialized Tera template service"
        );
        
        Ok(Self {
            tera: Arc::new(RwLock::new(tera)),
            config,
            template_dir,
        })
    }
    
    /// Initialize Tera template engine
    fn initialize_tera(template_dir: &Path) -> Result<Tera, DomainError> {
        let pattern = template_dir.join("**/*").to_string_lossy().to_string();
        
        match Tera::new(&pattern) {
            Ok(tera) => {
                debug!(
                    pattern = %pattern,
                    template_count = tera.get_template_names().count(),
                    "Successfully initialized Tera template engine"
                );
                Ok(tera)
            }
            Err(e) => {
                error!(
                    pattern = %pattern,
                    error = %e,
                    "Failed to initialize Tera template engine"
                );
                Err(DomainError::template_load_error(
                    format!("Failed to initialize Tera: {}", e)
                ))
            }
        }
    }
    
    /// Reload templates from disk
    pub async fn reload_templates(&self) -> Result<(), DomainError> {
        let new_tera = Self::initialize_tera(&self.template_dir)?;
        let mut tera = self.tera.write().await;
        *tera = new_tera;
        
        info!("Reloaded templates from disk");
        Ok(())
    }
    
    /// Build template filename for a given template name and mode
    fn build_template_filename(&self, template_name: &str, mode: &CommunicationMode, extension: &str) -> String {
        format!("{}_{}.{}", template_name, mode.to_string(), extension)
    }
    
    /// Get template paths for a given template name and mode
    fn get_template_paths(&self, template_name: &str, mode: &CommunicationMode) -> (String, Option<String>) {
        match mode {
            CommunicationMode::Email => {
                let html_template = self.build_template_filename(template_name, mode, &self.config.extensions.html);
                let text_template = self.build_template_filename(template_name, mode, &self.config.extensions.text);
                (text_template, Some(html_template))
            }
            _ => {
                let template = self.build_template_filename(template_name, mode, &self.config.extensions.text);
                (template, None)
            }
        }
    }
    
    /// Check if template files exist
    async fn template_files_exist(&self, template_name: &str, mode: &CommunicationMode) -> bool {
        let tera = self.tera.read().await;
        let (text_template, _html_template) = self.get_template_paths(template_name, mode);
        
        let text_exists = tera.get_template(&text_template).is_ok();
        
        match mode {
            CommunicationMode::Email => {
                // For email, we require at least the text template
                text_exists
            }
            _ => text_exists
        }
    }
    
    /// Render template with Tera
    async fn render_tera_template(&self, template_name: &str, variables: &HashMap<String, String>) -> Result<String, DomainError> {
        let tera = self.tera.read().await;
        
        let mut context = Context::new();
        for (key, value) in variables {
            context.insert(key, value);
        }
        
        tera.render(template_name, &context)
            .map_err(|e| DomainError::template_render_error(
                format!("Failed to render template '{}': {}", template_name, e)
            ))
    }
}

#[async_trait]
impl TemplateService for TeraTemplateService {
    async fn get_template(&self, name: &str, mode: &CommunicationMode) -> Result<MessageTemplate, DomainError> {
        if !self.template_files_exist(name, mode).await {
            return Err(DomainError::template_not_found(
                format!("Template '{}' for mode '{}' not found", name, mode)
            ));
        }
        
        // Create a basic MessageTemplate for file-based templates
        // Note: This is a simplified approach - in a full implementation,
        // you might want to read template metadata from companion files
        let content = match mode {
            CommunicationMode::Email => {
                TemplateContent::Email {
                    subject: format!("{} Subject", name), // This could be read from a metadata file
                    html_body: Some("HTML content placeholder".to_string()),
                    text_body: "Text content placeholder".to_string(),
                }
            }
            CommunicationMode::Notification => {
                TemplateContent::Notification {
                    title: format!("{} Notification", name),
                    body: "Notification body placeholder".to_string(),
                    default_data: HashMap::new(),
                }
            }
            CommunicationMode::Sms => {
                TemplateContent::Sms {
                    text: "SMS text placeholder".to_string(),
                }
            }
        };
        
        let template = MessageTemplate::new(name.to_string(), mode.clone(), content)?;
        
        Ok(template)
    }
    
    async fn find_template(&self, event_type: &str, mode: &CommunicationMode) -> Result<String, DomainError> {
        // Build the expected template name using the event type and mode
        let template_name = format!("{}_{}", event_type, mode.to_string());
        
        // Check if template files exist for this template name
        if self.template_files_exist(&template_name, mode).await {
            debug!(
                event_type = %event_type,
                mode = %mode,
                template_name = %template_name,
                "Found template for event type"
            );
            Ok(template_name)
        } else {
            // If the full template name doesn't exist, try just the event type
            if self.template_files_exist(event_type, mode).await {
                debug!(
                    event_type = %event_type,
                    mode = %mode,
                    template_name = %event_type,
                    "Found template using event type directly"
                );
                Ok(event_type.to_string())
            } else {
                error!(
                    event_type = %event_type,
                    mode = %mode,
                    template_dir = %self.config.template_dir,
                    "No template found for event type"
                );
                Err(DomainError::template_not_found(
                    format!("No template found for event type '{}' and mode '{}'", event_type, mode)
                ))
            }
        }
    }
    
    async fn render_template(
        &self,
        template_name: &str,
        mode: &CommunicationMode,
        variables: &HashMap<String, String>,
    ) -> Result<RenderedTemplate, DomainError> {
        if !self.template_files_exist(template_name, mode).await {
            return Err(DomainError::template_not_found(
                format!("Template '{}' for mode '{}' not found", template_name, mode)
            ));
        }
        
        let (text_template, html_template) = self.get_template_paths(template_name, mode);
        
        match mode {
            CommunicationMode::Email => {
                // Render text template (required)
                let text_body = self.render_tera_template(&text_template, variables).await?;
                
                // Render HTML template (optional)
                let html_body = if let Some(html_template_name) = html_template {
                    match self.render_tera_template(&html_template_name, variables).await {
                        Ok(html) => Some(html),
                        Err(e) => {
                            warn!(
                                template = %html_template_name,
                                error = %e,
                                "Failed to render HTML template, using text only"
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                
                // For email, we need a subject. In a real implementation, this could be:
                // 1. Read from a metadata file
                // 2. Extracted from the first line of the template
                // 3. Configured separately
                // For now, we'll use a simple approach
                let subject = variables.get("subject")
                    .unwrap_or(&format!("{} Email", template_name))
                    .clone();
                
                Ok(RenderedTemplate::Email {
                    subject,
                    html_body,
                    text_body,
                })
            }
            CommunicationMode::Notification => {
                let body = self.render_tera_template(&text_template, variables).await?;
                let title = variables.get("title")
                    .unwrap_or(&format!("{} Notification", template_name))
                    .clone();
                
                Ok(RenderedTemplate::Notification {
                    title,
                    body,
                    data: HashMap::new(),
                })
            }
            CommunicationMode::Sms => {
                let text = self.render_tera_template(&text_template, variables).await?;
                
                Ok(RenderedTemplate::Sms {
                    text,
                })
            }
        }
    }
    
    async fn create_template(&self, _template: MessageTemplate) -> Result<(), DomainError> {
        // File-based templates are created by placing files in the template directory
        // This operation is not supported for file-based templates
        Err(DomainError::operation_not_supported(
            "Creating templates is not supported for file-based template service. Place template files in the template directory instead.".to_string()
        ))
    }
    
    async fn update_template(&self, _id: Uuid, _template: MessageTemplate) -> Result<(), DomainError> {
        // File-based templates are updated by modifying files in the template directory
        // This operation is not supported for file-based templates
        Err(DomainError::operation_not_supported(
            "Updating templates is not supported for file-based template service. Modify template files in the template directory instead.".to_string()
        ))
    }
    
    async fn delete_template(&self, _id: Uuid) -> Result<(), DomainError> {
        // File-based templates are deleted by removing files from the template directory
        // This operation is not supported for file-based templates
        Err(DomainError::operation_not_supported(
            "Deleting templates is not supported for file-based template service. Remove template files from the template directory instead.".to_string()
        ))
    }
    
    async fn list_templates(&self, mode: Option<&CommunicationMode>) -> Result<Vec<MessageTemplate>, DomainError> {
        let tera = self.tera.read().await;
        let template_names: Vec<&str> = tera.get_template_names().collect();
        
        let mut templates = Vec::new();
        
        // Parse template names to find unique template prefixes
        let mut template_map: HashMap<String, Vec<&str>> = HashMap::new();
        
        for template_name in template_names {
            // Parse filename: template_prefix_mode.extension
            if let Some(stem) = Path::new(template_name).file_stem() {
                if let Some(stem_str) = stem.to_str() {
                    let parts: Vec<&str> = stem_str.rsplitn(2, '_').collect();
                    if parts.len() == 2 {
                        let template_prefix = parts[1];
                        template_map.entry(template_prefix.to_string())
                            .or_insert_with(Vec::new)
                            .push(template_name);
                    }
                }
            }
        }
        
        // Create MessageTemplate objects for each unique template
        for (template_prefix, _files) in template_map {
            let modes = if let Some(filter_mode) = mode {
                vec![filter_mode.clone()]
            } else {
                vec![CommunicationMode::Email, CommunicationMode::Notification, CommunicationMode::Sms]
            };
            
            for template_mode in modes {
                if self.template_files_exist(&template_prefix, &template_mode).await {
                    match self.get_template(&template_prefix, &template_mode).await {
                        Ok(template) => templates.push(template),
                        Err(e) => {
                            warn!(
                                template = %template_prefix,
                                mode = %template_mode,
                                error = %e,
                                "Failed to load template"
                            );
                        }
                    }
                }
            }
        }
        
        Ok(templates)
    }
    
    async fn template_exists(&self, name: &str, mode: &CommunicationMode) -> Result<bool, DomainError> {
        Ok(self.template_files_exist(name, mode).await)
    }
} 