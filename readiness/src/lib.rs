//! Shared `/ready` contract and queue-factory signaling for AIForAll services.
//!
//! RustyCog queue factories can degrade to a no-op without failing startup.
//! Composition roots must classify that outcome, log it, and expose it on
//! `GET /ready` so liveness (`/health`) is not mistaken for a live broker.

#![allow(missing_docs)]

pub mod classify;
pub mod factory;
pub mod http;
pub mod probe;

pub use classify::{
    classify_consumer, classify_publisher, classify_transport, signal_queue_status,
    ComponentStatus, QueueKind, QueueRole,
};
pub use factory::{
    create_signaled_event_consumer, create_signaled_multi_queue_event_publisher, SignaledConsumer,
    SignaledPublisher,
};
pub use http::attach_ready;
pub use probe::{CheckReport, ReadinessProbe, ReadinessReport};
