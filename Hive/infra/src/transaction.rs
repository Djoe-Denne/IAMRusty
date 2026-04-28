use async_trait::async_trait;
use hive_application::{ApplicationError, HiveOutboxUnitOfWork};
use rustycog_db::DbConnectionPool;
use rustycog_events::DomainEvent;
use rustycog_outbox::OutboxRecorder;

#[derive(Clone)]
pub struct HiveOutboxUnitOfWorkImpl {
    db: DbConnectionPool,
    outbox: OutboxRecorder,
}

impl HiveOutboxUnitOfWorkImpl {
    pub const fn new(db: DbConnectionPool, outbox: OutboxRecorder) -> Self {
        Self { db, outbox }
    }
}

#[async_trait]
impl HiveOutboxUnitOfWork for HiveOutboxUnitOfWorkImpl {
    async fn record_event(
        &self,
        event: Box<dyn DomainEvent + 'static>,
    ) -> Result<(), ApplicationError> {
        let txn = self.db.begin_write_transaction().await.map_err(|e| {
            ApplicationError::internal_error(&format!("failed to begin outbox transaction: {e}"))
        })?;

        let result = self.outbox.record(&txn, &event).await.map_err(|e| {
            ApplicationError::internal_error(&format!("failed to record Hive outbox event: {e}"))
        });

        match result {
            Ok(()) => {
                txn.commit().await.map_err(|e| {
                    ApplicationError::internal_error(&format!(
                        "failed to commit outbox transaction: {e}"
                    ))
                })?;
                Ok(())
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback Hive outbox transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }
}
