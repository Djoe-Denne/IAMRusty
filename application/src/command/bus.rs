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

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;
    use rstest::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use uuid::Uuid;

    // Test fixtures
    #[fixture]
    fn sample_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 2,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            use_jitter: false, // Disable jitter for predictable tests
        }
    }

    #[fixture]
    fn sample_command_bus_config(sample_retry_policy: RetryPolicy) -> CommandBusConfig {
        CommandBusConfig {
            default_timeout: Duration::from_millis(500),
            retry_policy: sample_retry_policy,
            enable_metrics: true,
            enable_tracing: true,
        }
    }

    // Mock metrics collector for testing
    #[derive(Debug, Default)]
    struct MockMetricsCollector {
        recorded_metrics: Arc<Mutex<Vec<CommandMetrics>>>,
    }

    impl MockMetricsCollector {
        fn new() -> Self {
            Self {
                recorded_metrics: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_recorded_metrics(&self) -> Vec<CommandMetrics> {
            self.recorded_metrics.lock().unwrap().clone()
        }

        fn clear_metrics(&self) {
            self.recorded_metrics.lock().unwrap().clear();
        }
    }

    #[async_trait]
    impl MetricsCollector for MockMetricsCollector {
        async fn record_metrics(&self, metrics: CommandMetrics) {
            self.recorded_metrics.lock().unwrap().push(metrics);
        }
    }

    // Test command for testing
    #[derive(Debug, Clone)]
    struct TestCommand {
        id: Uuid,
        data: String,
        should_fail_validation: bool,
    }

    impl TestCommand {
        fn new(data: String) -> Self {
            Self {
                id: Uuid::new_v4(),
                data,
                should_fail_validation: false,
            }
        }

        fn new_invalid() -> Self {
            Self {
                id: Uuid::new_v4(),
                data: "".to_string(),
                should_fail_validation: true,
            }
        }
    }

    #[async_trait]
    impl Command for TestCommand {
        type Result = String;

        fn command_type(&self) -> &'static str {
            "test_command"
        }

        fn command_id(&self) -> Uuid {
            self.id
        }

        fn validate(&self) -> Result<(), CommandError> {
            if self.should_fail_validation || self.data.is_empty() {
                Err(CommandError::Validation("Data cannot be empty".to_string()))
            } else {
                Ok(())
            }
        }
    }

    // Test command handler with configurable behavior
    struct TestCommandHandler {
        behavior: HandlerBehavior,
        call_count: Arc<Mutex<u32>>,
    }

    #[derive(Debug, Clone)]
    enum HandlerBehavior {
        Success,
        BusinessError,
        InfrastructureError,
        Timeout,
        SucceedAfterRetries(u32),
    }

    impl TestCommandHandler {
        fn new(behavior: HandlerBehavior) -> Self {
            Self {
                behavior,
                call_count: Arc::new(Mutex::new(0)),
            }
        }

        fn get_call_count(&self) -> u32 {
            *self.call_count.lock().unwrap()
        }
    }

    #[async_trait]
    impl CommandHandler<TestCommand> for TestCommandHandler {
        async fn handle(&self, command: TestCommand) -> Result<String, CommandError> {
            let current_count = {
                let mut count = self.call_count.lock().unwrap();
                *count += 1;
                *count
            }; // Mutex guard is dropped here

            match &self.behavior {
                HandlerBehavior::Success => Ok(format!("Processed: {}", command.data)),
                HandlerBehavior::BusinessError => Err(CommandError::Business("Business error".to_string())),
                HandlerBehavior::InfrastructureError => Err(CommandError::Infrastructure("Infrastructure error".to_string())),
                HandlerBehavior::Timeout => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Ok("Should not reach here".to_string())
                }
                HandlerBehavior::SucceedAfterRetries(succeed_after) => {
                    if current_count <= *succeed_after {
                        Err(CommandError::Infrastructure("Temporary failure".to_string()))
                    } else {
                        Ok(format!("Succeeded after {} attempts", current_count))
                    }
                }
            }
        }
    }

    mod retry_policy {
        use super::*;

        #[test]
        fn default_retry_policy_has_expected_values() {
            let policy = RetryPolicy::default();
            
            assert_eq!(policy.max_attempts, 3);
            assert_eq!(policy.base_delay, Duration::from_millis(100));
            assert_eq!(policy.max_delay, Duration::from_secs(30));
            assert_eq!(policy.backoff_multiplier, 2.0);
            assert!(policy.use_jitter);
        }

        #[rstest]
        #[test]
        fn calculate_delay_without_jitter(sample_retry_policy: RetryPolicy) {
            let delay1 = sample_retry_policy.calculate_delay(1);
            let delay2 = sample_retry_policy.calculate_delay(2);
            
            // With backoff multiplier of 2.0 and base delay of 50ms
            assert_eq!(delay1, Duration::from_millis(100)); // 50 * 2^1
            assert_eq!(delay2, Duration::from_millis(200)); // 50 * 2^2
        }

        #[test]
        fn calculate_delay_respects_max_delay() {
            let policy = RetryPolicy {
                max_attempts: 10,
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_millis(500),
                backoff_multiplier: 2.0,
                use_jitter: false,
            };
            
            let delay = policy.calculate_delay(10); // Would be 100 * 2^10 = 102400ms
            assert_eq!(delay, Duration::from_millis(500)); // Capped at max_delay
        }

        #[rstest]
        #[case(CommandError::Infrastructure("test".to_string()), true)]
        #[case(CommandError::Timeout, true)]
        #[case(CommandError::Validation("test".to_string()), false)]
        #[case(CommandError::Business("test".to_string()), false)]
        #[case(CommandError::RetryExhausted("test".to_string()), false)]
        #[test]
        fn is_retryable_identifies_retryable_errors(
            sample_retry_policy: RetryPolicy,
            #[case] error: CommandError,
            #[case] expected: bool
        ) {
            assert_eq!(sample_retry_policy.is_retryable(&error), expected);
        }
    }

    mod command_bus_config {
        use super::*;

        #[test]
        fn default_config_has_expected_values() {
            let config = CommandBusConfig::default();
            
            assert_eq!(config.default_timeout, Duration::from_secs(30));
            assert!(config.enable_metrics);
            assert!(config.enable_tracing);
        }

        #[rstest]
        #[test]
        fn config_stores_custom_values(sample_command_bus_config: CommandBusConfig) {
            assert_eq!(sample_command_bus_config.default_timeout, Duration::from_millis(500));
            assert_eq!(sample_command_bus_config.retry_policy.max_attempts, 2);
        }

        #[test]
        fn config_is_cloneable() {
            let config = CommandBusConfig::default();
            let cloned = config.clone();
            
            assert_eq!(cloned.default_timeout, config.default_timeout);
            assert_eq!(cloned.enable_metrics, config.enable_metrics);
        }
    }

    mod metrics_collector {
        use super::*;

        #[tokio::test]
        async fn mock_metrics_collector_records_metrics() {
            let collector = MockMetricsCollector::new();
            
            let metrics = CommandMetrics {
                command_type: "test".to_string(),
                duration_ms: 100,
                success: true,
                retry_attempts: 0,
                error_type: None,
            };
            
            collector.record_metrics(metrics.clone()).await;
            
            let recorded = collector.get_recorded_metrics();
            assert_eq!(recorded.len(), 1);
            assert_eq!(recorded[0].command_type, "test");
            assert_eq!(recorded[0].duration_ms, 100);
        }

        #[tokio::test]
        async fn logging_metrics_collector_doesnt_panic() {
            let collector = LoggingMetricsCollector;
            
            let metrics = CommandMetrics {
                command_type: "test".to_string(),
                duration_ms: 100,
                success: true,
                retry_attempts: 0,
                error_type: None,
            };
            
            // Should not panic
            collector.record_metrics(metrics).await;
        }
    }

    mod command_bus {
        use super::*;

        #[test]
        fn new_creates_bus_with_default_config() {
            let _bus = CommandBus::new();
            // Can't directly test private fields, but creation should succeed
            assert!(true);
        }

        #[tokio::test]
        async fn execute_successful_command() {
            let config = CommandBusConfig {
                default_timeout: Duration::from_secs(1),
                retry_policy: RetryPolicy::default(),
                enable_metrics: false, // Disable for simpler test
                enable_tracing: false,
            };
            
            let bus = CommandBus::with_config(config);
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::Success));
            let command = TestCommand::new("test data".to_string());
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler.clone(), context).await;
            
            assert_ok!(&result);
            assert_eq!(result.unwrap(), "Processed: test data");
            assert_eq!(handler.get_call_count(), 1);
        }

        #[tokio::test]
        async fn execute_command_with_validation_error() {
            let bus = CommandBus::new();
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::Success));
            let command = TestCommand::new_invalid();
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler.clone(), context).await;
            
            assert_err!(&result);
            if let Err(CommandError::Validation(msg)) = result {
                assert!(msg.contains("Data cannot be empty"));
            } else {
                panic!("Expected validation error");
            }
            assert_eq!(handler.get_call_count(), 0); // Handler should not be called
        }

        #[tokio::test]
        async fn execute_command_with_business_error_no_retry() {
            let bus = CommandBus::new();
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::BusinessError));
            let command = TestCommand::new("test data".to_string());
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler.clone(), context).await;
            
            assert_err!(&result);
            if let Err(CommandError::Business(msg)) = result {
                assert_eq!(msg, "Business error");
            } else {
                panic!("Expected business error");
            }
            assert_eq!(handler.get_call_count(), 1); // Should not retry business errors
        }

        #[tokio::test]
        async fn execute_command_with_infrastructure_error_retries() {
            let config = CommandBusConfig {
                default_timeout: Duration::from_secs(1),
                retry_policy: RetryPolicy {
                    max_attempts: 2,
                    base_delay: Duration::from_millis(10),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                    use_jitter: false,
                },
                enable_metrics: false,
                enable_tracing: false,
            };
            
            let bus = CommandBus::with_config(config);
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::InfrastructureError));
            let command = TestCommand::new("test data".to_string());
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler.clone(), context).await;
            
            assert_err!(&result);
            assert_eq!(handler.get_call_count(), 2); // Initial + 1 retry (max_attempts = 2)
        }

        #[tokio::test]
        async fn execute_command_succeeds_after_retries() {
            let config = CommandBusConfig {
                default_timeout: Duration::from_secs(1),
                retry_policy: RetryPolicy {
                    max_attempts: 3,
                    base_delay: Duration::from_millis(10),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                    use_jitter: false,
                },
                enable_metrics: false,
                enable_tracing: false,
            };
            
            let bus = CommandBus::with_config(config);
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::SucceedAfterRetries(2)));
            let command = TestCommand::new("test data".to_string());
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler.clone(), context).await;
            
            assert_ok!(&result);
            assert_eq!(result.unwrap(), "Succeeded after 3 attempts");
            assert_eq!(handler.get_call_count(), 3); // Failed twice, succeeded on third
        }

        #[tokio::test]
        async fn execute_command_records_success_metrics() {
            let collector = Arc::new(MockMetricsCollector::new());
            let config = CommandBusConfig {
                default_timeout: Duration::from_secs(1),
                retry_policy: RetryPolicy::default(),
                enable_metrics: true,
                enable_tracing: false,
            };
            
            let bus = CommandBus::with_metrics_collector(config, collector.clone());
            let handler = Arc::new(TestCommandHandler::new(HandlerBehavior::Success));
            let command = TestCommand::new("test data".to_string());
            let context = CommandContext::new();
            
            let result = bus.execute(command, handler, context).await;
            
            assert_ok!(&result);
            
            let metrics = collector.get_recorded_metrics();
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics[0].command_type, "test_command");
            assert!(metrics[0].success);
            assert_eq!(metrics[0].retry_attempts, 0);
            assert!(metrics[0].error_type.is_none());
        }
    }
} 