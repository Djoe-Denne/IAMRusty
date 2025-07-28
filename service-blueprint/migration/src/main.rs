use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = {{SERVICE_NAME}}_configuration::load_config()?;
    
    // Build database URL
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.database.username,
        config.database.password,
        config.database.host,
        config.database.port,
        config.database.name
    );

    // Run migrations
    cli::run_cli({{SERVICE_NAME}}_migration::Migrator, &database_url).await?;
    
    Ok(())
} 