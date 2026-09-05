//! `EventHandler` implementation that wires the translators, idempotency
//! ledger, and `OpenFGA` write client together.

use std::sync::Arc;

use async_trait::async_trait;
use rustycog::core::error::ServiceError;
use rustycog::events::{DomainEvent, EventHandler};
use serde::Deserialize;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::fga_client::OpenFgaWriteClient;
use crate::idempotency::EventLedger;
use crate::translator::{Translator, TupleDelta};

pub struct SyncEventHandler {
    translators: Vec<Arc<dyn Translator>>,
    ledger: Arc<dyn EventLedger>,
    fga: OpenFgaWriteClient,
}

#[derive(Deserialize)]
struct VisibilityChangeOrder {
    project_id: Uuid,
    visibility_revision: i64,
}

impl SyncEventHandler {
    pub fn new(
        translators: Vec<Arc<dyn Translator>>,
        ledger: Arc<dyn EventLedger>,
        fga: OpenFgaWriteClient,
    ) -> Self {
        Self {
            translators,
            ledger,
            fga,
        }
    }

    /// Try every translator in order until one claims the event. Returns the
    /// resulting delta (possibly empty) or `None` if no translator recognized
    /// the payload.
    fn translate(&self, raw_event: &serde_json::Value) -> Option<(TupleDelta, &'static str)> {
        for translator in &self.translators {
            match translator.translate(raw_event) {
                Ok(Some(delta)) => return Some((delta, translator.name())),
                Ok(None) => {}
                Err(e) => warn!(translator = translator.name(), error = %e, "translator error"),
            }
        }
        None
    }

    fn visibility_change_order(
        raw_event: &serde_json::Value,
    ) -> Result<Option<VisibilityChangeOrder>, ServiceError> {
        if raw_event
            .get("event_type")
            .and_then(serde_json::Value::as_str)
            != Some("project_visibility_changed")
        {
            return Ok(None);
        }
        serde_json::from_value(raw_event.clone())
            .map(Some)
            .map_err(|error| ServiceError::internal(format!("visibility event decode: {error}")))
    }
}

#[async_trait]
impl EventHandler for SyncEventHandler {
    async fn handle_event(&self, event: Box<dyn DomainEvent>) -> Result<(), ServiceError> {
        let event_id = event.event_id();
        let event_type = event.event_type().to_string();

        let should_process = self
            .ledger
            .begin(event_id)
            .await
            .map_err(|e| ServiceError::internal(format!("ledger.begin failed: {e}")))?;
        if !should_process {
            debug!(event_id = %event_id, event_type = %event_type, "completed duplicate event, skipping");
            return Ok(());
        }

        let raw = event.to_json().and_then(|s| {
            serde_json::from_str::<serde_json::Value>(&s)
                .map_err(|e| ServiceError::internal(format!("event json decode: {e}")))
        })?;
        let visibility_order = Self::visibility_change_order(&raw)?;
        if let Some(order) = &visibility_order {
            let is_next = self
                .ledger
                .begin_visibility_change(order.project_id, order.visibility_revision)
                .await
                .map_err(|error| {
                    ServiceError::infrastructure(format!(
                        "visibility revision is not ready for processing: {error}"
                    ))
                })?;
            if !is_next {
                debug!(
                    event_id = %event_id,
                    project_id = %order.project_id,
                    revision = order.visibility_revision,
                    "obsolete visibility event, skipping"
                );
                self.ledger.complete(event_id).await.map_err(|error| {
                    ServiceError::internal(format!("ledger.complete failed: {error}"))
                })?;
                return Ok(());
            }
        }

        let Some((delta, translator_name)) = self.translate(&raw) else {
            debug!(event_id = %event_id, event_type = %event_type, "no translator claimed event");
            self.ledger
                .complete(event_id)
                .await
                .map_err(|e| ServiceError::internal(format!("ledger.complete failed: {e}")))?;
            return Ok(());
        };

        if delta.is_empty() {
            debug!(
                event_id = %event_id,
                event_type = %event_type,
                translator = translator_name,
                "translator produced empty delta"
            );
            if let Some(order) = &visibility_order {
                self.ledger
                    .complete_visibility_change(order.project_id, order.visibility_revision)
                    .await
                    .map_err(|error| {
                        ServiceError::internal(format!(
                            "visibility revision completion failed: {error}"
                        ))
                    })?;
            }
            self.ledger.complete(event_id).await.map_err(|error| {
                ServiceError::internal(format!("ledger.complete failed: {error}"))
            })?;
            return Ok(());
        }

        if let Err(error) = self.fga.write(&delta.writes, &delta.deletes).await {
            let error_message = format!("OpenFGA write failed: {error}");
            if let Err(ledger_error) = self.ledger.fail(event_id, &error_message).await {
                warn!(event_id = %event_id, error = %ledger_error, "failed to mark event delivery as failed");
            }
            return Err(ServiceError::infrastructure(&error_message));
        }

        if let Some(order) = &visibility_order {
            self.ledger
                .complete_visibility_change(order.project_id, order.visibility_revision)
                .await
                .map_err(|error| {
                    ServiceError::internal(format!(
                        "visibility revision completion failed: {error}"
                    ))
                })?;
        }

        self.ledger
            .complete(event_id)
            .await
            .map_err(|e| ServiceError::internal(format!("ledger.complete failed: {e}")))?;

        info!(
            event_id = %event_id,
            event_type = %event_type,
            translator = translator_name,
            writes = delta.writes.len(),
            deletes = delta.deletes.len(),
            "applied tuple delta"
        );
        Ok(())
    }

    fn supports_event_type(&self, _event_type: &str) -> bool {
        // The worker accepts every event and lets translators self-select;
        // unknown events are silently skipped above.
        true
    }
}
