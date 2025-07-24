use async_trait::async_trait;

#[async_trait]
pub trait ServiceTestDescriptor<T>: Send + Sync + 'static {
    type Config: rustycog_config::ConfigLoader<Self::Config> + rustycog_config::HasServerConfig + rustycog_config::HasLoggingConfig + rustycog_config::HasDbConfig + Send + Sync + 'static;
    
    async fn build_app(&self, config: Self::Config) -> anyhow::Result<()>;
    
    async fn run_app(&self, server_config: rustycog_config::ServerConfig) -> anyhow::Result<()>;

    async fn run_migrations(&self, connection: &sea_orm::DatabaseConnection) -> anyhow::Result<()>;

    fn has_db(&self) -> bool;

    fn has_sqs(&self) -> bool;
    
}