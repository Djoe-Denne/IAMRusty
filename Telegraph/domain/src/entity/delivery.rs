//! Message delivery domain entities for Telegraph communication service

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use super::message::CommunicationMode;

/// Message delivery record tracking the status of a sent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelivery {
    /// Delivery record ID
    pub id: Uuid,
    /// ID of the message being delivered
    pub message_id: Uuid,
    /// Communication mode used
    pub mode: CommunicationMode,
    /// Current delivery status
    pub status: DeliveryStatus,
    /// Number of delivery attempts
    pub attempts: u32,
    /// External provider message ID (if available)
    pub provider_message_id: Option<String>,
    /// Delivery metadata
    pub metadata: HashMap<String, String>,
    /// When delivery was first attempted
    pub created_at: DateTime<Utc>,
    /// When delivery status was last updated
    pub updated_at: DateTime<Utc>,
    /// When message was successfully delivered (if applicable)
    pub delivered_at: Option<DateTime<Utc>>,
    /// Error details if delivery failed
    pub error_details: Option<String>,
}

/// Message delivery status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    /// Message is pending delivery
    Pending,
    /// Message is being processed for delivery
    Processing,
    /// Message has been sent to the provider
    Sent,
    /// Message has been delivered to the recipient
    Delivered,
    /// Message delivery failed
    Failed,
    /// Message delivery was rejected by the provider
    Rejected,
    /// Message bounced back
    Bounced,
    /// Message was read by the recipient (if supported)
    Read,
}

/// Delivery attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryAttempt {
    /// Attempt ID
    pub id: Uuid,
    /// Delivery record this attempt belongs to
    pub delivery_id: Uuid,
    /// Attempt number
    pub attempt_number: u32,
    /// Status of this attempt
    pub status: DeliveryStatus,
    /// When the attempt was made
    pub attempted_at: DateTime<Utc>,
    /// Error message if the attempt failed
    pub error_message: Option<String>,
    /// Response from the communication provider
    pub provider_response: Option<String>,
    /// Duration of the attempt in milliseconds
    pub duration_ms: Option<u64>,
}

impl MessageDelivery {
    /// Create a new message delivery record
    pub fn new(message_id: Uuid, mode: CommunicationMode) -> Self {
        let now = Utc::now();
        
        Self {
            id: Uuid::new_v4(),
            message_id,
            mode,
            status: DeliveryStatus::Pending,
            attempts: 0,
            provider_message_id: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            delivered_at: None,
            error_details: None,
        }
    }
    
    /// Mark delivery as processing
    pub fn mark_processing(&mut self) {
        self.status = DeliveryStatus::Processing;
        self.updated_at = Utc::now();
    }
    
    /// Mark delivery as sent with provider message ID
    pub fn mark_sent(&mut self, provider_message_id: Option<String>) {
        self.status = DeliveryStatus::Sent;
        self.provider_message_id = provider_message_id;
        self.updated_at = Utc::now();
        self.attempts += 1;
    }
    
    /// Mark delivery as successful
    pub fn mark_delivered(&mut self) {
        self.status = DeliveryStatus::Delivered;
        self.delivered_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.error_details = None;
    }
    
    /// Mark delivery as failed
    pub fn mark_failed(&mut self, error_details: String) {
        self.status = DeliveryStatus::Failed;
        self.error_details = Some(error_details);
        self.updated_at = Utc::now();
        self.attempts += 1;
    }
    
    /// Mark delivery as rejected
    pub fn mark_rejected(&mut self, error_details: String) {
        self.status = DeliveryStatus::Rejected;
        self.error_details = Some(error_details);
        self.updated_at = Utc::now();
        self.attempts += 1;
    }
    
    /// Mark delivery as bounced
    pub fn mark_bounced(&mut self, error_details: String) {
        self.status = DeliveryStatus::Bounced;
        self.error_details = Some(error_details);
        self.updated_at = Utc::now();
    }
    
    /// Mark message as read
    pub fn mark_read(&mut self) {
        self.status = DeliveryStatus::Read;
        self.updated_at = Utc::now();
    }
    
    /// Add metadata to the delivery record
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
        self.updated_at = Utc::now();
    }
    
    /// Check if delivery can be retried
    pub fn can_retry(&self, max_attempts: u32) -> bool {
        matches!(self.status, DeliveryStatus::Failed) && self.attempts < max_attempts
    }
    
    /// Check if delivery is in a final state
    pub fn is_final(&self) -> bool {
        matches!(
            self.status,
            DeliveryStatus::Delivered | DeliveryStatus::Rejected | DeliveryStatus::Bounced | DeliveryStatus::Read
        )
    }
    
    /// Check if delivery was successful
    pub fn is_successful(&self) -> bool {
        matches!(self.status, DeliveryStatus::Delivered | DeliveryStatus::Read)
    }
    
    /// Get delivery duration if available
    pub fn delivery_duration(&self) -> Option<chrono::Duration> {
        self.delivered_at.map(|delivered| delivered - self.created_at)
    }
}

impl DeliveryAttempt {
    /// Create a new delivery attempt
    pub fn new(delivery_id: Uuid, attempt_number: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            delivery_id,
            attempt_number,
            status: DeliveryStatus::Processing,
            attempted_at: Utc::now(),
            error_message: None,
            provider_response: None,
            duration_ms: None,
        }
    }
    
    /// Mark attempt as successful
    pub fn mark_success(&mut self, provider_response: Option<String>, duration_ms: Option<u64>) {
        self.status = DeliveryStatus::Sent;
        self.provider_response = provider_response;
        self.duration_ms = duration_ms;
    }
    
    /// Mark attempt as failed
    pub fn mark_failed(&mut self, error_message: String, duration_ms: Option<u64>) {
        self.status = DeliveryStatus::Failed;
        self.error_message = Some(error_message);
        self.duration_ms = duration_ms;
    }
    
    /// Mark attempt as rejected
    pub fn mark_rejected(&mut self, error_message: String, duration_ms: Option<u64>) {
        self.status = DeliveryStatus::Rejected;
        self.error_message = Some(error_message);
        self.duration_ms = duration_ms;
    }
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryStatus::Pending => write!(f, "pending"),
            DeliveryStatus::Processing => write!(f, "processing"),
            DeliveryStatus::Sent => write!(f, "sent"),
            DeliveryStatus::Delivered => write!(f, "delivered"),
            DeliveryStatus::Failed => write!(f, "failed"),
            DeliveryStatus::Rejected => write!(f, "rejected"),
            DeliveryStatus::Bounced => write!(f, "bounced"),
            DeliveryStatus::Read => write!(f, "read"),
        }
    }
} 