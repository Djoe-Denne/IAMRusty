//! Test-specific app builder that allows injecting a mock event publisher
//! 
//! This module provides functionality to build the application for tests
//! with a custom event publisher for verifying event publishing behavior.

use std::sync::Arc;
use anyhow::Result;
use configuration::AppConfig;
use rustycog_http::AppState;
use setup::app::build_app_state_with_event_publisher;
use crate::common::mock_event_publisher::MockEventPublisher;

/// Build app state with a mock event publisher for testing
pub async fn build_test_app_state_with_mock_events(
    config: AppConfig,
    mock_event_publisher: Arc<MockEventPublisher>,
) -> Result<(AppState, Arc<MockEventPublisher>)> {
    // Use the existing app building logic with our custom event publisher
    // The generic function can now accept our concrete MockEventPublisher type
    let app_state = build_app_state_with_event_publisher(config, mock_event_publisher.clone()).await?;
    
    Ok((app_state, mock_event_publisher))
}

/// Build app state with a new mock event publisher for testing
pub async fn build_test_app_state_with_new_mock_events(
    config: AppConfig,
) -> Result<(AppState, Arc<MockEventPublisher>)> {
    let mock_event_publisher = Arc::new(MockEventPublisher::new());
    build_test_app_state_with_mock_events(config, mock_event_publisher).await
} 