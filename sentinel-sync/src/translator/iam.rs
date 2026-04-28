//! IAM event -> OpenFGA tuple translation.
//!
//! Populated by the `iam-translator` todo if any IAM events end up feeding
//! authorization facts. Most IAM events are lifecycle notifications the
//! authorization model does not need to react to.

use anyhow::Result;
use iam_events::IamDomainEvent;

use super::{Translator, TupleDelta};

#[derive(Default)]
pub struct IamTranslator;

impl IamTranslator {
    pub fn new() -> Self {
        Self
    }
}

impl Translator for IamTranslator {
    fn name(&self) -> &'static str {
        "iam"
    }

    fn translate(&self, raw_event: &serde_json::Value) -> Result<Option<TupleDelta>> {
        let event: IamDomainEvent = match serde_json::from_value(raw_event.clone()) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        match event {
            IamDomainEvent::UserSignedUp(_)
            | IamDomainEvent::UserEmailVerified(_)
            | IamDomainEvent::UserLoggedIn(_)
            | IamDomainEvent::PasswordResetRequested(_) => Ok(Some(TupleDelta::default())),
        }
    }
}
