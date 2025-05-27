use super::{Command, CommandError, CommandHandler, CommandContext, CommandMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{info, warn, error, instrument};

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to use jitter
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Calculate delay for the given attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay.as_millis() as f64 
            * self.backoff_multiplier.powi(attempt as i32);
        
        let delay = Duration::from_millis(delay as u64).min(self.max_delay);
        
        if self.use_jitter {
            // Add up to 25% jitter
            let jitter = fastrand::f64() * 0.25;
            let jitter_delay = delay.as_millis() as f64 * (1.0 + jitter);
            Duration::from_millis(jitter_delay as u64)
        } else {
            delay
        }
    }
    
    /// Check if an error is retryable
    pub fn is_retryable(&self, error: &CommandError) -> bool {
        matches!(error, 
            CommandError::Infrastructure(_) | 
            CommandError::Timeout
        )
    }
}

/// Command bus configuration
#[derive(Debug, Clone)]
pub struct CommandBusConfig {
    /// Default timeout for command execution
    pub default_timeout: Duration,
    /// Default retry policy
    pub retry_policy: RetryPolicy,
    /// Whether to enable metrics collection
    pub enable_metrics: bool,
    /// Whether to enable detailed tracing
    pub enable_tracing: bool,
}

impl Default for CommandBusConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            enable_metrics: true,
            enable_tracing: true,
        }
    }
}

/// Metrics collector trait
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Record command execution metrics
    async fn record_metrics(&self, metrics: CommandMetrics);
}

/// Default metrics collector that logs metrics
pub struct LoggingMetricsCollector;

#[async_trait]
impl MetricsCollector for LoggingMetricsCollector {
    async fn record_metrics(&self, metrics: CommandMetrics) {
        if metrics.success {
            info!(
                command_type = %metrics.command_type,
                duration_ms = metrics.duration_ms,
                retry_attempts = metrics.retry_attempts,
                "Command executed successfully"
            );
        } else {
            warn!(
                command_type = %metrics.command_type,
                duration_ms = metrics.duration_ms,
                retry_attempts = metrics.retry_attempts,
                error_type = ?metrics.error_type,
                "Command execution failed"
            );
        }
    }
}

/// Command bus for executing commands with cross-cutting concerns
pub struct CommandBus {
    config: CommandBusConfig,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl CommandBus {
    /// Create a new command bus with default configuration
    pub fn new() -> Self {
        Self {
            config: CommandBusConfig::default(),
            metrics_collector: Arc::new(LoggingMetricsCollector),
        }
    }
    
    /// Create a new command bus with custom configuration
    pub fn with_config(config: CommandBusConfig) -> Self {
        Self {
            config,
            metrics_collector: Arc::new(LoggingMetricsCollector),
        }
    }
    
    /// Create a new command bus with custom metrics collector
    pub fn with_metrics_collector(
        config: CommandBusConfig,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            config,
            metrics_collector,
        }
    }
    
    /// Execute a command with the given handler
    #[instrument(
        skip(self, handler, command),
        fields(
            command_type = command.command_type(),
            command_id = %command.command_id(),
            execution_id = %context.execution_id,
            user_id = ?context.user_id,
            request_id = ?context.request_id
        )
    )]
    pub async fn execute<C, H>(
        &self,
        command: C,
        handler: Arc<H>,
        context: CommandContext,
    ) -> Result<C::Result, CommandError>
    where
        C: Command + Clone,
        H: CommandHandler<C>,
    {
        let start_time = Instant::now();
        let mut retry_attempts = 0;
        
        // Validate command first
        if let Err(e) = command.validate() {
            self.record_failure_metrics(&command, start_time, retry_attempts, &e).await;
            return Err(e);
        }
        
        info!("Executing command: {}", command.command_type());
        
        loop {
            let _attempt_start = Instant::now();
            
            // Execute command with timeout
            let result = timeout(
                self.config.default_timeout,
                handler.handle(command.clone())
            ).await;
            
            match result {
                Ok(Ok(result)) => {
                    // Success
                    let duration = start_time.elapsed();
                    info!(
                        duration_ms = duration.as_millis() as u64,
                        retry_attempts = retry_attempts,
                        "Command executed successfully"
                    );
                    
                    if self.config.enable_metrics {
                        self.record_success_metrics(&command, duration, retry_attempts).await;
                    }
                    
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    // Command failed
                    retry_attempts += 1;
                    
                    // Check if error is retryable first
                    if !self.config.retry_policy.is_retryable(&e) {
                        // Error is not retryable - return original error immediately
                        let duration = start_time.elapsed();
                        error!(
                            error = %e,
                            duration_ms = duration.as_millis() as u64,
                            retry_attempts = retry_attempts,
                            "Command execution failed - error not retryable"
                        );
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &e).await;
                        }
                        
                        return Err(e);
                    }
                    
                    // Error is retryable - check if we've reached max attempts
                    if retry_attempts >= self.config.retry_policy.max_attempts {
                        // Max retries reached - return RetryExhausted
                        let duration = start_time.elapsed();
                        error!(
                            error = %e,
                            duration_ms = duration.as_millis() as u64,
                            retry_attempts = retry_attempts,
                            "Command execution failed after maximum retries"
                        );
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &e).await;
                        }
                        
                        return Err(CommandError::RetryExhausted(e.to_string()));
                    }
                    
                    // Calculate delay and retry
                    let delay = self.config.retry_policy.calculate_delay(retry_attempts - 1);
                    warn!(
                        error = %e,
                        retry_attempt = retry_attempts,
                        delay_ms = delay.as_millis() as u64,
                        "Command failed, retrying"
                    );
                    
                    tokio::time::sleep(delay).await;
                }
                Err(_) => {
                    // Timeout
                    retry_attempts += 1;
                    let timeout_error = CommandError::Timeout;
                    
                    if retry_attempts >= self.config.retry_policy.max_attempts {
                        let duration = start_time.elapsed();
                        error!(
                            duration_ms = duration.as_millis() as u64,
                            retry_attempts = retry_attempts,
                            "Command execution timed out after retries"
                        );
                        
                        if self.config.enable_metrics {
                            self.record_failure_metrics(&command, start_time, retry_attempts, &timeout_error).await;
                        }
                        
                        return Err(CommandError::RetryExhausted("Timeout".to_string()));
                    }
                    
                    let delay = self.config.retry_policy.calculate_delay(retry_attempts - 1);
                    warn!(
                        retry_attempt = retry_attempts,
                        delay_ms = delay.as_millis() as u64,
                        "Command timed out, retrying"
                    );
                    
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    async fn record_success_metrics<C: Command>(
        &self,
        command: &C,
        duration: Duration,
        retry_attempts: u32,
    ) {
        let metrics = CommandMetrics {
            command_type: command.command_type().to_string(),
            duration_ms: duration.as_millis() as u64,
            success: true,
            retry_attempts,
            error_type: None,
        };
        
        self.metrics_collector.record_metrics(metrics).await;
    }
    
    async fn record_failure_metrics<C: Command>(
        &self,
        command: &C,
        start_time: Instant,
        retry_attempts: u32,
        error: &CommandError,
    ) {
        let duration = start_time.elapsed();
        let metrics = CommandMetrics {
            command_type: command.command_type().to_string(),
            duration_ms: duration.as_millis() as u64,
            success: false,
            retry_attempts,
            error_type: Some(format!("{:?}", error)),
        };
        
        self.metrics_collector.record_metrics(metrics).await;
    }
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
} 