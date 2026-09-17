//! Infrastructure layer for Lazaret.

pub mod connector_proxy;
pub mod enrollment_postgres;
pub mod event;
pub mod identity;
pub mod kv_postgres;
pub mod kv_redis;
pub mod manifesto_client;
pub mod secrets_deny;
pub mod vault;

pub use connector_proxy::NamedConnectorProxy;
pub use enrollment_postgres::PostgresEnrollmentRegistry;
pub use event::{KvPurgeEventConsumer, KvPurgeEventHandler};
pub use identity::{
    build_identity_service, DedicatedSessionSigner, InMemoryEnrollmentRegistry, PlatformInternalCa,
};
pub use kv_postgres::PostgresKvStore;
pub use kv_redis::RedisKvStore;
pub use manifesto_client::HttpBindingGrantClient;
pub use secrets_deny::DeniedSecretResolver;
pub use vault::VaultHttpSecretResolver;
