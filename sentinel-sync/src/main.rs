//! sentinel-sync worker entry point.
//!
//! Boots a single event consumer that receives domain events from Hive,
//! Manifesto, and IAM, feeds each event through the per-service translator,
//! and writes the resulting tuple deltas into OpenFGA.

use std::sync::Arc;

use anyhow::{Context, Result};
use rustycog_events::{create_event_consumer_from_queue_config, EventConsumer};
use tracing::{error, info};

mod config;
mod fga_client;
mod handler;
mod idempotency;
mod translator;

use crate::config::SentinelSyncConfig;
use crate::fga_client::OpenFgaWriteClient;
use crate::handler::SyncEventHandler;
use crate::idempotency::build_ledger;
use crate::translator::{hive::HiveTranslator, iam::IamTranslator, manifesto::ManifestoTranslator};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = SentinelSyncConfig::load().context("failed to load sentinel-sync config")?;
    info!(
        api_url = %config.openfga.api_url,
        store_id = %config.openfga.store_id,
        "sentinel-sync starting"
    );

    let fga = OpenFgaWriteClient::new(config.openfga.clone())?;
    let ledger: Arc<dyn idempotency::EventLedger> =
        Arc::from(build_ledger(&config.idempotency)?);

    let translators: Vec<Arc<dyn translator::Translator>> = vec![
        Arc::new(HiveTranslator::new()),
        Arc::new(ManifestoTranslator::new()),
        Arc::new(IamTranslator::new()),
    ];

    let handler = SyncEventHandler::new(translators, ledger, fga);

    let consumer = create_event_consumer_from_queue_config(&config.queue)
        .await
        .context("failed to create event consumer")?;

    // Start consuming; on shutdown signal, stop gracefully.
    let consumer_handle = {
        let consumer = consumer.clone();
        tokio::spawn(async move {
            if let Err(e) = consumer.start(handler).await {
                error!("event consumer failed: {e}");
                Err(e)
            } else {
                Ok(())
            }
        })
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown signal received");
        }
        result = consumer_handle => {
            match result {
                Ok(Ok(())) => info!("consumer completed"),
                Ok(Err(e)) => return Err(anyhow::anyhow!("consumer failed: {e}")),
                Err(e) => return Err(anyhow::anyhow!("consumer task panicked: {e}")),
            }
        }
    }

    if let Err(e) = consumer.stop().await {
        error!("failed to stop consumer: {e}");
    }

    info!("sentinel-sync stopped");
    Ok(())
}
