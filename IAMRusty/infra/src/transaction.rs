use async_trait::async_trait;
use iam_domain::{error::DomainError, service::IamOutboxUnitOfWork};
use rustycog_db::DbConnectionPool;
use rustycog_events::event::DomainEvent;
use rustycog_outbox::OutboxRecorder;

#[derive(Clone)]
pub struct IamOutboxUnitOfWorkImpl {
    db: DbConnectionPool,
    outbox: OutboxRecorder,
}

impl IamOutboxUnitOfWorkImpl {
    pub fn new(db: DbConnectionPool, outbox: OutboxRecorder) -> Self {
        Self { db, outbox }
    }
}

#[async_trait]
impl IamOutboxUnitOfWork for IamOutboxUnitOfWorkImpl {
    async fn record_event(&self, event: Box<dyn DomainEvent + 'static>) -> Result<(), DomainError> {
        let txn = self.db.begin_write_transaction().await.map_err(|e| {
            DomainError::RepositoryError(format!("failed to begin outbox transaction: {e}"))
        })?;

        let result = self.outbox.record(&txn, &event).await.map_err(|e| {
            DomainError::RepositoryError(format!("failed to record IAMRusty outbox event: {e}"))
        });

        match result {
            Ok(()) => {
                txn.commit().await.map_err(|e| {
                    DomainError::RepositoryError(format!(
                        "failed to commit outbox transaction: {e}"
                    ))
                })?;
                Ok(())
            }
            Err(error) => {
                if let Err(rollback_error) = txn.rollback().await {
                    tracing::error!(
                        "failed to rollback IAMRusty outbox transaction: {}",
                        rollback_error
                    );
                }
                Err(error)
            }
        }
    }
}
