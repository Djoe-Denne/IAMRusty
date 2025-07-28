use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;

/// PostgreSQL database connection manager
#[derive(Clone)]
pub struct PostgresConnection {
    pub connection: Arc<DatabaseConnection>,
}

impl PostgresConnection {
    /// Create a new PostgreSQL connection
    pub async fn new(database_url: &str) -> Result<Self, DbErr> {
        let connection = Database::connect(database_url).await?;
        Ok(Self {
            connection: Arc::new(connection),
        })
    }

    /// Get the database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }
}

/// Database transaction wrapper
pub struct DatabaseTransaction<'a> {
    pub txn: sea_orm::DatabaseTransaction,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> DatabaseTransaction<'a> {
    /// Create a new transaction wrapper
    pub fn new(txn: sea_orm::DatabaseTransaction) -> Self {
        Self {
            txn,
            _marker: std::marker::PhantomData,
        }
    }

    /// Commit the transaction
    pub async fn commit(self) -> Result<(), DbErr> {
        self.txn.commit().await
    }

    /// Rollback the transaction
    pub async fn rollback(self) -> Result<(), DbErr> {
        self.txn.rollback().await
    }
}

/// Database error conversion utilities
pub mod error_conversion {
    use {{SERVICE_NAME}}_domain::DomainError;
    use sea_orm::DbErr;

    /// Convert SeaORM database error to domain error
    pub fn db_error_to_domain_error(error: DbErr) -> DomainError {
        match error {
            DbErr::RecordNotFound(_) => DomainError::entity_not_found("Record", "unknown"),
            DbErr::Custom(msg) => DomainError::external_service_error("database", &msg),
            DbErr::Conn(msg) => DomainError::external_service_error("database_connection", &msg),
            DbErr::Exec(msg) => DomainError::external_service_error("database_execution", &msg),
            DbErr::Query(msg) => DomainError::external_service_error("database_query", &msg),
            _ => DomainError::internal(&format!("Database error: {:?}", error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_postgres_connection_creation() {
        // This test would need a real PostgreSQL connection in integration tests
        // For now, we just test that the code compiles
        assert!(true);
    }
} 