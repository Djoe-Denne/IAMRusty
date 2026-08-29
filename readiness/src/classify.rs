//! Classify rustycog queue factory results so a no-op is never silent.

use rustycog::config::QueueConfig;
use rustycog::events::{ConcreteEventConsumer, ConcreteEventPublisher};

/// Configured queue transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    /// Amazon SQS / LocalStack.
    Sqs,
    /// Apache Kafka.
    Kafka,
}

impl QueueKind {
    /// Stable wire name used in `/ready` JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sqs => "sqs",
            Self::Kafka => "kafka",
        }
    }

    /// Kind declared by `QueueConfig`, if any.
    #[must_use]
    pub const fn from_config(config: &QueueConfig) -> Option<Self> {
        match config {
            QueueConfig::Sqs(_) => Some(Self::Sqs),
            QueueConfig::Kafka(_) => Some(Self::Kafka),
            QueueConfig::Disabled => None,
        }
    }
}

/// Publisher versus consumer role in readiness output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueRole {
    /// Outbound event publisher (outbox dispatcher).
    Publisher,
    /// Inbound event consumer.
    Consumer,
}

impl QueueRole {
    /// Stable wire name used in `/ready` JSON and logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Publisher => "publisher",
            Self::Consumer => "consumer",
        }
    }
}

/// Outcome of inspecting a rustycog factory result against config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentStatus {
    /// Queue is disabled; no-op is intentional.
    Disabled,
    /// Test injection; transport is not the rustycog factory.
    Injected,
    /// Real Kafka/SQS client was constructed.
    Live {
        /// Transport actually constructed.
        kind: QueueKind,
    },
    /// Config asked for a live broker but the factory returned no-op.
    Degraded {
        /// Transport that was requested.
        expected: QueueKind,
        /// Stable machine-readable reason.
        reason: &'static str,
    },
}

impl ComponentStatus {
    /// Whether `/ready` must fail without a further transport ping.
    #[must_use]
    pub const fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded { .. })
    }
}

/// Inspect a concrete publisher against the service `QueueConfig`.
#[must_use]
pub fn classify_publisher(
    config: &QueueConfig,
    publisher: &ConcreteEventPublisher,
) -> ComponentStatus {
    match publisher {
        ConcreteEventPublisher::NoOp(_) => classify_noop(config),
        ConcreteEventPublisher::Sqs(_) => ComponentStatus::Live {
            kind: QueueKind::Sqs,
        },
        ConcreteEventPublisher::Kafka(_) => ComponentStatus::Live {
            kind: QueueKind::Kafka,
        },
    }
}

/// Inspect a concrete consumer against the service `QueueConfig`.
#[must_use]
pub fn classify_consumer(
    config: &QueueConfig,
    consumer: &ConcreteEventConsumer,
) -> ComponentStatus {
    match consumer {
        ConcreteEventConsumer::NoOp(_) => classify_noop(config),
        ConcreteEventConsumer::Sqs(_) => ComponentStatus::Live {
            kind: QueueKind::Sqs,
        },
        ConcreteEventConsumer::Kafka(_) => ComponentStatus::Live {
            kind: QueueKind::Kafka,
        },
    }
}

/// Classify from config + a no-op flag (when the concrete enum is not in hand).
#[must_use]
pub fn classify_transport(config: &QueueConfig, is_noop: bool) -> ComponentStatus {
    if is_noop {
        classify_noop(config)
    } else {
        ComponentStatus::Live {
            kind: QueueKind::from_config(config).unwrap_or(QueueKind::Sqs),
        }
    }
}

fn classify_noop(config: &QueueConfig) -> ComponentStatus {
    if config.is_enabled() {
        ComponentStatus::Degraded {
            expected: QueueKind::from_config(config).unwrap_or(QueueKind::Sqs),
            reason: "factory_fallback_noop",
        }
    } else {
        ComponentStatus::Disabled
    }
}

/// Log the factory outcome so a no-op is never silent.
pub fn signal_queue_status(service: &str, role: QueueRole, status: &ComponentStatus) {
    match status {
        ComponentStatus::Disabled => {
            tracing::info!(
                service,
                role = role.as_str(),
                "queue transport disabled (intentional no-op)"
            );
        }
        ComponentStatus::Injected => {
            tracing::info!(
                service,
                role = role.as_str(),
                "queue transport injected (test double)"
            );
        }
        ComponentStatus::Live { kind } => {
            tracing::info!(
                service,
                role = role.as_str(),
                transport = kind.as_str(),
                "queue transport live"
            );
        }
        ComponentStatus::Degraded { expected, reason } => {
            tracing::error!(
                service,
                role = role.as_str(),
                expected = expected.as_str(),
                reason,
                "queue factory degraded to no-op; /ready will fail until the broker is live"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustycog::config::{KafkaConfig, SqsConfig};
    use rustycog::events::NoOpEventPublisher;
    use std::sync::Arc;

    fn enabled_sqs() -> QueueConfig {
        let mut config = SqsConfig::default();
        config.enabled = true;
        QueueConfig::Sqs(config)
    }

    fn disabled_sqs() -> QueueConfig {
        let mut config = SqsConfig::default();
        config.enabled = false;
        QueueConfig::Sqs(config)
    }

    #[test]
    fn disabled_config_with_noop_is_intentional() {
        let publisher = ConcreteEventPublisher::NoOp(Arc::new(NoOpEventPublisher::new()));
        assert_eq!(
            classify_publisher(&QueueConfig::Disabled, &publisher),
            ComponentStatus::Disabled
        );
        assert_eq!(
            classify_publisher(&disabled_sqs(), &publisher),
            ComponentStatus::Disabled
        );
    }

    #[test]
    fn enabled_config_with_noop_is_degraded() {
        let publisher = ConcreteEventPublisher::NoOp(Arc::new(NoOpEventPublisher::new()));
        assert_eq!(
            classify_publisher(&enabled_sqs(), &publisher),
            ComponentStatus::Degraded {
                expected: QueueKind::Sqs,
                reason: "factory_fallback_noop",
            }
        );
    }

    #[test]
    fn classify_transport_matches_publisher_for_noop() {
        assert_eq!(
            classify_transport(&enabled_sqs(), true),
            classify_publisher(
                &enabled_sqs(),
                &ConcreteEventPublisher::NoOp(Arc::new(NoOpEventPublisher::new()))
            )
        );
    }

    #[test]
    fn enabled_kafka_noop_names_kafka() {
        let mut kafka = KafkaConfig::default();
        kafka.enabled = true;
        assert_eq!(
            classify_transport(&QueueConfig::Kafka(kafka), true),
            ComponentStatus::Degraded {
                expected: QueueKind::Kafka,
                reason: "factory_fallback_noop",
            }
        );
    }
}
