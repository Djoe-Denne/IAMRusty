//! Telegraph event -> `OpenFGA` tuple translation.
//!
//! `NotificationCreated` writes `notification:{id}#recipient@user:{user_id}`
//! so the HTTP mark-read guard can resolve `Write` on `"notification"`.

use anyhow::Result;
use telegraph_events::TelegraphDomainEvent;

use super::{Translator, TupleDelta};
use crate::fga_client::Tuple;

#[derive(Default)]
pub struct TelegraphTranslator;

impl TelegraphTranslator {
    pub const fn new() -> Self {
        Self
    }
}

impl Translator for TelegraphTranslator {
    fn name(&self) -> &'static str {
        "telegraph"
    }

    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>> {
        let event: TelegraphDomainEvent = match serde_json::from_value(raw_event.clone()) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        let delta = match event {
            TelegraphDomainEvent::NotificationCreated(evt) => {
                TupleDelta::default().write(Tuple::user(
                    "notification",
                    evt.notification_id,
                    "recipient",
                    evt.user_id,
                ))
            }
        };

        Ok(Some(delta))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use telegraph_events::NotificationCreatedEvent;
    use uuid::Uuid;

    fn to_json<T: serde::Serialize>(value: T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn notification_created_writes_recipient_tuple() {
        let notification_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let evt = TelegraphDomainEvent::NotificationCreated(NotificationCreatedEvent::new(
            notification_id,
            user_id,
            Utc::now(),
        ));
        let delta = TelegraphTranslator::new()
            .translate(&to_json(evt))
            .unwrap()
            .unwrap();
        assert_eq!(delta.writes.len(), 1);
        assert_eq!(delta.writes[0].object_type, "notification");
        assert_eq!(delta.writes[0].relation, "recipient");
        assert_eq!(delta.writes[0].object_id, notification_id.to_string());
        assert_eq!(delta.writes[0].user_id, user_id.to_string());
        assert!(delta.deletes.is_empty());
    }
}
