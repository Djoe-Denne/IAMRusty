//! `EventHandler` implementation that wires the translators, idempotency
//! ledger, and OpenFGA write client together.

use std::sync::Arc;

use async_trait::async_trait;
use rustycog_core::error::ServiceError;
use rustycog_events::{DomainEvent, EventHandler};
use tracing::{debug, info, warn};

use crate::fga_client::OpenFgaWriteClient;
use crate::idempotency::EventLedger;
use crate::translator::{Translator, TupleDelta};

pub struct SyncEventHandler {
    translators: Vec<Arc<dyn Translator>>,
    ledger: Arc<dyn EventLedger>,
    fga: OpenFgaWriteClient,
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
                Ok(None) => continue,
                Err(e) => warn!(translator = translator.name(), error = %e, "translator error"),
            }
        }
        None
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
            .map_err(|e| ServiceError::internal(&format!("ledger.begin failed: {e}")))?;
        if !should_process {
            debug!(event_id = %event_id, event_type = %event_type, "completed duplicate event, skipping");
            return Ok(());
        }

        let raw = event.to_json().and_then(|s| {
            serde_json::from_str::<serde_json::Value>(&s)
                .map_err(|e| ServiceError::internal(&format!("event json decode: {e}")))
        })?;

        let Some((delta, translator_name)) = self.translate(&raw) else {
            debug!(event_id = %event_id, event_type = %event_type, "no translator claimed event");
            self.ledger
                .complete(event_id)
                .await
                .map_err(|e| ServiceError::internal(&format!("ledger.complete failed: {e}")))?;
            return Ok(());
        };

        if delta.is_empty() {
            debug!(
                event_id = %event_id,
                event_type = %event_type,
                translator = translator_name,
                "translator produced empty delta"
            );
            self.ledger
                .complete(event_id)
                .await
                .map_err(|e| ServiceError::internal(&format!("ledger.complete failed: {e}")))?;
            return Ok(());
        }

        if let Err(error) = self.fga.write(&delta.writes, &delta.deletes).await {
            let error_message = format!("OpenFGA write failed: {error}");
            if let Err(ledger_error) = self.ledger.fail(event_id, &error_message).await {
                warn!(event_id = %event_id, error = %ledger_error, "failed to mark event delivery as failed");
            }
            return Err(ServiceError::infrastructure(&error_message));
        }

        self.ledger
            .complete(event_id)
            .await
            .map_err(|e| ServiceError::internal(&format!("ledger.complete failed: {e}")))?;

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
