//! Compose database + queue checks into a `/ready` report.

use std::collections::BTreeMap;
use std::sync::Arc;

use rustycog::events::{ConcreteEventConsumer, ConcreteEventPublisher, EventConsumer, EventPublisher};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde::Serialize;

use crate::classify::{ComponentStatus, QueueKind};

/// Shared readiness probe wired into each service HTTP stack.
pub struct ReadinessProbe {
    service: &'static str,
    kind: ProbeKind,
}

enum ProbeKind {
    Service {
        database: Option<Arc<DatabaseConnection>>,
        publisher: Option<QueueAttachment>,
        consumer: Option<QueueAttachment>,
    },
    Aggregate {
        children: Vec<(&'static str, Arc<ReadinessProbe>)>,
    },
}

struct QueueAttachment {
    status: ComponentStatus,
    publisher: Option<Arc<ConcreteEventPublisher>>,
    consumer: Option<Arc<ConcreteEventConsumer>>,
}

/// JSON body returned by `GET /ready`.
#[derive(Debug, Clone, Serialize)]
pub struct ReadinessReport {
    /// Overall readiness (`ready` / `not_ready`).
    pub status: &'static str,
    /// Service or aggregate name.
    pub service: &'static str,
    /// Individual checks, keyed by name.
    pub checks: BTreeMap<String, CheckReport>,
}

/// One check in a [`ReadinessReport`].
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    /// `ok`, `disabled`, or `error`.
    pub status: &'static str,
    /// Transport name when relevant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<&'static str>,
    /// Extra detail (degraded reason, ping error, injected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ReadinessProbe {
    /// Single-service probe. Add database / queue attachments with builders.
    #[must_use]
    pub fn new(service: &'static str) -> Self {
        Self {
            service,
            kind: ProbeKind::Service {
                database: None,
                publisher: None,
                consumer: None,
            },
        }
    }

    /// Monolith-style probe that ANDs child service reports.
    #[must_use]
    pub fn aggregate(service: &'static str, children: Vec<(&'static str, Arc<Self>)>) -> Self {
        Self {
            service,
            kind: ProbeKind::Aggregate { children },
        }
    }

    /// Ping the write connection on every `/ready` call.
    #[must_use]
    pub fn with_database(mut self, database: Arc<DatabaseConnection>) -> Self {
        if let ProbeKind::Service {
            database: slot, ..
        } = &mut self.kind
        {
            *slot = Some(database);
        }
        self
    }

    /// Attach the publisher factory outcome (and optional live transport for ping).
    #[must_use]
    pub fn with_publisher(
        mut self,
        status: ComponentStatus,
        transport: Option<Arc<ConcreteEventPublisher>>,
    ) -> Self {
        if let ProbeKind::Service { publisher, .. } = &mut self.kind {
            *publisher = Some(QueueAttachment {
                status,
                publisher: transport,
                consumer: None,
            });
        }
        self
    }

    /// Attach the consumer factory outcome (and optional live transport for ping).
    #[must_use]
    pub fn with_consumer(
        mut self,
        status: ComponentStatus,
        transport: Option<Arc<ConcreteEventConsumer>>,
    ) -> Self {
        if let ProbeKind::Service { consumer, .. } = &mut self.kind {
            *consumer = Some(QueueAttachment {
                status,
                publisher: None,
                consumer: transport,
            });
        }
        self
    }

    /// Evaluate all checks.
    pub async fn report(&self) -> ReadinessReport {
        match &self.kind {
            ProbeKind::Service {
                database,
                publisher,
                consumer,
            } => self.service_report(database.as_ref(), publisher.as_ref(), consumer.as_ref()).await,
            ProbeKind::Aggregate { children } => self.aggregate_report(children).await,
        }
    }

    /// Whether every check is `ok` or `disabled`.
    pub async fn is_ready(&self) -> bool {
        self.report().await.status == "ready"
    }

    async fn service_report(
        &self,
        database: Option<&Arc<DatabaseConnection>>,
        publisher: Option<&QueueAttachment>,
        consumer: Option<&QueueAttachment>,
    ) -> ReadinessReport {
        let mut checks = BTreeMap::new();
        if let Some(database) = database {
            checks.insert("database".to_owned(), database_check(database.as_ref()).await);
        }
        if let Some(publisher) = publisher {
            checks.insert(
                "queue_publisher".to_owned(),
                queue_check(publisher).await,
            );
        }
        if let Some(consumer) = consumer {
            checks.insert("queue_consumer".to_owned(), queue_check(consumer).await);
        }
        Self::finish(self.service, checks)
    }

    async fn aggregate_report(&self, children: &[(&'static str, Arc<Self>)]) -> ReadinessReport {
        let mut checks = BTreeMap::new();
        for (name, child) in children {
            let child_report = Box::pin(child.report()).await;
            let ready = child_report.status == "ready";
            checks.insert(
                (*name).to_owned(),
                CheckReport {
                    status: if ready { "ok" } else { "error" },
                    transport: None,
                    detail: (!ready).then(|| {
                        child_report
                            .checks
                            .iter()
                            .filter(|(_, check)| check.status == "error")
                            .map(|(key, check)| {
                                check.detail.as_deref().map_or_else(
                                    || (*key).clone(),
                                    |detail| format!("{key}: {detail}"),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    }),
                },
            );
        }
        Self::finish(self.service, checks)
    }

    fn finish(service: &'static str, checks: BTreeMap<String, CheckReport>) -> ReadinessReport {
        let ready = checks.values().all(|check| check.status != "error");
        ReadinessReport {
            status: if ready { "ready" } else { "not_ready" },
            service,
            checks,
        }
    }
}

async fn database_check(database: &DatabaseConnection) -> CheckReport {
    match database.ping().await {
        Ok(()) => CheckReport {
            status: "ok",
            transport: None,
            detail: None,
        },
        Err(error) => CheckReport {
            status: "error",
            transport: None,
            detail: Some(error.to_string()),
        },
    }
}

async fn queue_check(attachment: &QueueAttachment) -> CheckReport {
    match &attachment.status {
        ComponentStatus::Disabled => CheckReport {
            status: "disabled",
            transport: None,
            detail: None,
        },
        ComponentStatus::Injected => CheckReport {
            status: "ok",
            transport: None,
            detail: Some("injected".to_owned()),
        },
        ComponentStatus::Degraded { expected, reason } => CheckReport {
            status: "error",
            transport: Some(expected.as_str()),
            detail: Some((*reason).to_owned()),
        },
        ComponentStatus::Live { kind } => ping_live(*kind, attachment).await,
    }
}

async fn ping_live(kind: QueueKind, attachment: &QueueAttachment) -> CheckReport {
    let ping_error = if let Some(publisher) = &attachment.publisher {
        publisher
            .health_check()
            .await
            .err()
            .map(|error| error.to_string())
    } else if let Some(consumer) = &attachment.consumer {
        consumer
            .health_check()
            .await
            .err()
            .map(|error| error.to_string())
    } else {
        None
    };

    if let Some(detail) = ping_error {
        CheckReport {
            status: "error",
            transport: Some(kind.as_str()),
            detail: Some(detail),
        }
    } else {
        CheckReport {
            status: "ok",
            transport: Some(kind.as_str()),
            detail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_queue_is_ready() {
        let probe = ReadinessProbe::new("manifesto")
            .with_publisher(ComponentStatus::Disabled, None)
            .with_consumer(ComponentStatus::Disabled, None);
        let report = probe.report().await;
        assert_eq!(report.status, "ready");
        assert_eq!(report.checks["queue_publisher"].status, "disabled");
        assert_eq!(report.checks["queue_consumer"].status, "disabled");
    }

    #[tokio::test]
    async fn degraded_publisher_is_not_ready() {
        let probe = ReadinessProbe::new("hive").with_publisher(
            ComponentStatus::Degraded {
                expected: QueueKind::Sqs,
                reason: "factory_fallback_noop",
            },
            None,
        );
        let report = probe.report().await;
        assert_eq!(report.status, "not_ready");
        assert_eq!(report.checks["queue_publisher"].status, "error");
        assert_eq!(
            report.checks["queue_publisher"].detail.as_deref(),
            Some("factory_fallback_noop")
        );
    }

    #[tokio::test]
    async fn aggregate_fails_when_a_child_fails() {
        let ok = Arc::new(ReadinessProbe::new("iam").with_publisher(ComponentStatus::Disabled, None));
        let bad = Arc::new(ReadinessProbe::new("hive").with_publisher(
            ComponentStatus::Degraded {
                expected: QueueKind::Sqs,
                reason: "factory_fallback_noop",
            },
            None,
        ));
        let probe = ReadinessProbe::aggregate("monolith", vec![("iam", ok), ("hive", bad)]);
        let report = probe.report().await;
        assert_eq!(report.status, "not_ready");
        assert_eq!(report.checks["iam"].status, "ok");
        assert_eq!(report.checks["hive"].status, "error");
    }

    #[tokio::test]
    async fn injected_publisher_is_ready() {
        let probe =
            ReadinessProbe::new("iam").with_publisher(ComponentStatus::Injected, None);
        let report = probe.report().await;
        assert_eq!(report.status, "ready");
        assert_eq!(
            report.checks["queue_publisher"].detail.as_deref(),
            Some("injected")
        );
    }
}
