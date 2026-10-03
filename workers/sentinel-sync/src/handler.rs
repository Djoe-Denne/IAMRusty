//! `EventHandler` implementation that wires the translators, idempotency
//! ledger, and `OpenFGA` write client together.

use std::sync::Arc;

use async_trait::async_trait;
use rustycog::core::error::ServiceError;
use rustycog::events::{DomainEvent, EventHandler};
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

#[derive(Debug, Clone, Copy)]
struct LifecycleOrder {
    project_id: Uuid,
    revision: i64,
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
}

/// Rebuild `{ event_type, data }` when the transport delivered a flat payload.
#[must_use]
pub fn canonical_event_envelope(event_type: &str, raw: serde_json::Value) -> serde_json::Value {
    if raw
        .get("event_type")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && raw.get("data").is_some()
    {
        return raw;
    }
    serde_json::json!({
        "event_type": event_type,
        "data": raw,
    })
}

#[must_use]
pub fn is_manifesto_event_type(event_type: &str) -> bool {
    event_type.starts_with("project_")
        || event_type.starts_with("member_")
        || event_type.starts_with("permission_")
        || event_type.starts_with("component_")
}

fn payload_object(raw: &serde_json::Value) -> &serde_json::Value {
    raw.get("data").unwrap_or(raw)
}

fn lifecycle_order(raw: &serde_json::Value) -> Result<Option<LifecycleOrder>, ServiceError> {
    let event_type = raw
        .get("event_type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ServiceError::internal("event envelope missing event_type"))?;
    let payload = payload_object(raw);
    let revision_field = match event_type {
        "project_visibility_changed" => "visibility_revision",
        "project_suspended" | "project_resumed" | "project_archived" => "lifecycle_revision",
        _ => return Ok(None),
    };
    let revision = payload
        .get(revision_field)
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if revision <= 0 {
        return Ok(None);
    }
    let project_id = payload
        .get("project_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ServiceError::internal("lifecycle event missing project_id"))?;
    let project_id = Uuid::parse_str(project_id)
        .map_err(|error| ServiceError::internal(format!("lifecycle project_id: {error}")))?;
    Ok(Some(LifecycleOrder {
        project_id,
        revision,
    }))
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
        let raw = canonical_event_envelope(&event_type, raw);
        let visibility_order = lifecycle_order(&raw)?;
        if let Some(order) = &visibility_order {
            let is_next = self
                .ledger
                .begin_visibility_change(order.project_id, order.revision)
                .await
                .map_err(|error| {
                    ServiceError::infrastructure(format!(
                        "lifecycle revision is not ready for processing: {error}"
                    ))
                })?;
            if !is_next {
                debug!(
                    event_id = %event_id,
                    project_id = %order.project_id,
                    revision = order.revision,
                    "obsolete lifecycle event, skipping"
                );
                self.ledger.complete(event_id).await.map_err(|error| {
                    ServiceError::internal(format!("ledger.complete failed: {error}"))
                })?;
                return Ok(());
            }
        }

        let Some((delta, translator_name)) = self.translate(&raw) else {
            if is_manifesto_event_type(&event_type) {
                let error_message = format!("manifesto event {event_type} could not be decoded");
                if let Err(ledger_error) = self.ledger.fail(event_id, &error_message).await {
                    warn!(event_id = %event_id, error = %ledger_error, "failed to mark event delivery as failed");
                }
                return Err(ServiceError::internal(error_message));
            }
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
                    .complete_visibility_change(order.project_id, order.revision)
                    .await
                    .map_err(|error| {
                        ServiceError::internal(format!(
                            "lifecycle revision completion failed: {error}"
                        ))
                    })?;
            }
            self.ledger.complete(event_id).await.map_err(|error| {
                ServiceError::internal(format!("ledger.complete failed: {error}"))
            })?;
            return Ok(());
        }

        if let Err(error) = self
            .fga
            .write_idempotent(&delta.writes, &delta.deletes)
            .await
        {
            let error_message = format!("OpenFGA write failed: {error}");
            if let Err(ledger_error) = self.ledger.fail(event_id, &error_message).await {
                warn!(event_id = %event_id, error = %ledger_error, "failed to mark event delivery as failed");
            }
            return Err(ServiceError::infrastructure(&error_message));
        }

        if let Some(order) = &visibility_order {
            self.ledger
                .complete_visibility_change(order.project_id, order.revision)
                .await
                .map_err(|error| {
                    ServiceError::internal(format!("lifecycle revision completion failed: {error}"))
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
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use manifesto_events::{ManifestoDomainEvent, ProjectCreatedEvent};
    use rustycog::events::DomainEvent;

    #[test]
    fn wraps_flat_kafka_payload() {
        let project_id = Uuid::new_v4();
        let flat = serde_json::json!({
            "event_id": Uuid::new_v4(),
            "aggregate_id": project_id,
            "event_type": "member_removed",
            "project_id": project_id,
            "member_id": Uuid::new_v4(),
            "user_id": Uuid::new_v4(),
            "removed_by": Uuid::new_v4(),
            "removed_at": Utc::now(),
        });
        let wrapped = canonical_event_envelope("member_removed", flat);
        assert_eq!(
            wrapped
                .get("event_type")
                .and_then(serde_json::Value::as_str),
            Some("member_removed")
        );
        assert!(wrapped.get("data").is_some());
    }

    #[test]
    fn keeps_already_enveloped_payload() {
        let evt = ManifestoDomainEvent::ProjectCreated(ProjectCreatedEvent::new(
            Uuid::new_v4(),
            "demo".into(),
            "personal".into(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "private".into(),
            Utc::now(),
        ));
        let raw: serde_json::Value = serde_json::from_str(&evt.to_json().unwrap()).unwrap();
        let wrapped = canonical_event_envelope("project_created", raw.clone());
        assert_eq!(wrapped, raw);
    }

    #[test]
    fn manifesto_undecodable_types_are_detected() {
        assert!(is_manifesto_event_type("project_created"));
        assert!(is_manifesto_event_type("member_removed"));
        assert!(is_manifesto_event_type("permission_revoked"));
        assert!(is_manifesto_event_type("component_added"));
        assert!(!is_manifesto_event_type("organization_created"));
    }

    #[test]
    fn lifecycle_order_reads_visibility_and_status_revisions() {
        let project_id = Uuid::new_v4();
        let visibility = serde_json::json!({
            "event_type": "project_visibility_changed",
            "data": {
                "project_id": project_id,
                "visibility_revision": 4
            }
        });
        let order = lifecycle_order(&visibility).unwrap().unwrap();
        assert_eq!(order.revision, 4);
        let suspend = serde_json::json!({
            "event_type": "project_suspended",
            "data": {
                "project_id": project_id,
                "lifecycle_revision": 7
            }
        });
        let order = lifecycle_order(&suspend).unwrap().unwrap();
        assert_eq!(order.revision, 7);
        let v1 = serde_json::json!({
            "event_type": "project_archived",
            "data": { "project_id": project_id }
        });
        assert!(lifecycle_order(&v1).unwrap().is_none());
    }

    use std::collections::HashMap;
    use std::sync::Arc;

    use manifesto_events::{
        MemberRemovedEvent, PermissionGrantedEvent, ProjectVisibilityChangedEvent,
    };
    use rustycog::config::OpenFgaClientConfig;
    use rustycog::core::error::ServiceError;
    use rustycog::events::EventHandler;

    use crate::fga_client::OpenFgaWriteClient;
    use crate::idempotency::{EventLedger, InMemoryEventLedger};
    use crate::translator::manifesto::ManifestoTranslator;

    #[derive(Debug)]
    struct JsonDomainEvent {
        event_id: Uuid,
        event_type: String,
        aggregate_id: Uuid,
        json: serde_json::Value,
    }

    impl DomainEvent for JsonDomainEvent {
        fn event_type(&self) -> &str {
            &self.event_type
        }

        fn event_id(&self) -> Uuid {
            self.event_id
        }

        fn aggregate_id(&self) -> Uuid {
            self.aggregate_id
        }

        fn occurred_at(&self) -> chrono::DateTime<Utc> {
            Utc::now()
        }

        fn version(&self) -> u32 {
            1
        }

        fn to_json(&self) -> Result<String, ServiceError> {
            serde_json::to_string(&self.json)
                .map_err(|error| ServiceError::internal(format!("json: {error}")))
        }

        fn metadata(&self) -> HashMap<String, String> {
            HashMap::new()
        }
    }

    fn dummy_handler() -> (SyncEventHandler, Arc<InMemoryEventLedger>) {
        let ledger = Arc::new(InMemoryEventLedger::new());
        let fga = OpenFgaWriteClient::new(OpenFgaClientConfig {
            scheme: "http".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1,
            store_id: "test-store".to_string(),
            authorization_model_id: None,
            api_token: None,
            cache_ttl_seconds: Some(0),
        })
        .expect("dummy fga");
        let handler = SyncEventHandler::new(
            vec![Arc::new(ManifestoTranslator::new())],
            ledger.clone(),
            fga,
        );
        (handler, ledger)
    }

    #[tokio::test]
    async fn flat_v1_member_removed_is_noop_and_completes() {
        let (handler, ledger) = dummy_handler();
        let inner = MemberRemovedEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Utc::now(),
        );
        let event_id = inner.base.event_id;
        let event = JsonDomainEvent {
            event_id,
            event_type: "member_removed".to_string(),
            aggregate_id: inner.project_id,
            json: serde_json::to_value(&inner).expect("flat payload"),
        };
        handler
            .handle_event(Box::new(event))
            .await
            .expect("v1 remove is a no-op");
        assert!(!ledger.begin(event_id).await.unwrap());
    }

    #[tokio::test]
    async fn enveloped_v1_member_removed_is_noop() {
        let (handler, _) = dummy_handler();
        let event = ManifestoDomainEvent::MemberRemoved(MemberRemovedEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        handler
            .handle_event(Box::new(event))
            .await
            .expect("enveloped v1 no-op");
    }

    #[tokio::test]
    async fn undecodable_manifesto_event_is_not_completed() {
        let (handler, ledger) = dummy_handler();
        let event_id = Uuid::new_v4();
        let event = JsonDomainEvent {
            event_id,
            event_type: "project_created".to_string(),
            aggregate_id: Uuid::new_v4(),
            json: serde_json::json!({ "garbage": true }),
        };
        assert!(handler.handle_event(Box::new(event)).await.is_err());
        assert!(ledger.begin(event_id).await.unwrap());
    }

    #[tokio::test]
    async fn later_lifecycle_revision_wins_and_older_is_skipped() {
        let (handler, _) = dummy_handler();
        let project_id = Uuid::new_v4();
        let newer = ManifestoDomainEvent::ProjectVisibilityChanged(
            ProjectVisibilityChangedEvent::new(
                project_id,
                "personal".into(),
                Uuid::new_v4(),
                "private".into(),
                "private".into(),
                Uuid::new_v4(),
                Utc::now(),
            )
            .with_visibility_revision(5),
        );
        handler
            .handle_event(Box::new(newer))
            .await
            .expect("newer revision");
        let older = ManifestoDomainEvent::ProjectVisibilityChanged(
            ProjectVisibilityChangedEvent::new(
                project_id,
                "personal".into(),
                Uuid::new_v4(),
                "private".into(),
                "internal".into(),
                Uuid::new_v4(),
                Utc::now(),
            )
            .with_visibility_revision(2),
        );
        handler
            .handle_event(Box::new(older))
            .await
            .expect("older revision skipped");
    }

    #[tokio::test]
    async fn v2_permission_granted_needs_openfga_and_stays_retryable_on_failure() {
        let (handler, ledger) = dummy_handler();
        let event = ManifestoDomainEvent::PermissionGranted(PermissionGrantedEvent::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "project".into(),
            "read".into(),
            Uuid::new_v4(),
            Utc::now(),
        ));
        let event_id = event.event_id();
        assert!(handler.handle_event(Box::new(event)).await.is_err());
        assert!(ledger.begin(event_id).await.unwrap());
    }
}
