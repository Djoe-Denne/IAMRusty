use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use rustycog_config::HasLoggingConfig;

#[cfg(feature = "scaleway-loki")]
use std::env;
#[cfg(feature = "scaleway-loki")]
use rustycog_config::{HasScalewayConfig};

#[cfg(feature = "scaleway-loki")]
pub trait ServiceLoggerConfig: HasLoggingConfig + HasScalewayConfig {}
#[cfg(feature = "scaleway-loki")]
impl<T: HasLoggingConfig + HasScalewayConfig> ServiceLoggerConfig for T {}

#[cfg(not(feature = "scaleway-loki"))]
pub trait ServiceLoggerConfig: HasLoggingConfig {}
#[cfg(not(feature = "scaleway-loki"))]
impl<T: HasLoggingConfig> ServiceLoggerConfig for T {}


/// Setup logging based on configuration
pub fn setup_logging<C: ServiceLoggerConfig>(config: &C)
{
    let log_level = match config.logging_config().level.to_lowercase().as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        _ => LevelFilter::INFO,
    };

    let console_layer = tracing_subscriber::fmt::layer().with_line_number(true).with_target(true).with_thread_names(true);
    
    #[cfg(feature = "scaleway-loki")] 
    let (loki_layer, loki_task) = if let Some(scaleway_loki) = config.logging_config().scaleway_loki.clone() {
            let loki_endpoint = format!("https://{}.logs.cockpit.{}.scw.cloud", scaleway_loki.datasource_uuid, config.scaleway_config().region);
            let (loki_layer, loki_task) = tracing_loki::builder()
                .label("job", env::var("JOB").unwrap_or_else(|_| "unknown".to_string())) // TODO: add job label from environnement variable
                .expect("Failed to set job label")
                .label("service", env::var("SERVICE").unwrap_or_else(|_| "unknown".to_string())) // TODO: add service label from envvar
                .expect("Failed to set service label")
                .http_header("Authorization", format!("Bearer {}", scaleway_loki.cockpit_token))
                .expect("Failed to set Authorization header")
                .build_url(loki_endpoint.parse().expect("Failed to parse Loki endpoint"))
                .expect("Failed to build Loki layer");

            (Some(loki_layer), Some(loki_task))
        } else {
            (None, None)
        };

    #[cfg(not(feature = "scaleway-loki"))]
    let loki_layer: Option<tracing_subscriber::fmt::Layer<_>> = None;

    // Use try_init() to avoid panicking if subscriber is already initialized
    // This is especially important during testing where setup_logging might be called multiple times
    let _ = tracing_subscriber::registry()
        .with(log_level)
        .with(console_layer)
        .with(loki_layer)
        .try_init();

    #[cfg(feature = "scaleway-loki")] {
        if let Some(loki_task) = loki_task {    
            // Spawn the Loki background task
            tokio::spawn(loki_task);
        }
    }
}
