//! Standalone startup and owned-listener interfaces; all service operations refuse in T0.

use std::fmt;

use oauth_connect_configuration::AppConfig;
use rustycog::core::error::DomainError;
use rustycog::http::OwnedHttpServer;

/// Opaque, fully initialized application, created before admitting connections.
pub struct Application {
    _private: (),
}

impl fmt::Debug for Application {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Application([REDACTED])")
    }
}

impl Application {
    /// Prepare the whole mixed catalogue and HTTP state before any listener bind.
    ///
    /// # Errors
    /// Always refuses in T0 without resolving secrets, building clients or routing.
    pub async fn new(_config: AppConfig) -> Result<Self, DomainError> {
        std::future::ready(Err(pending())).await
    }

    /// Start an SDK-owned listener, whose effective addresses support port zero.
    ///
    /// The future implementation uses SDK owned startup; callers retain explicit
    /// shutdown/join ownership. Only successful binds may produce address logs.
    ///
    /// # Errors
    /// Always refuses in T0 without binding a socket or preparing TLS.
    pub async fn start(&self) -> Result<OwnedHttpServer, DomainError> {
        std::future::ready(Err(pending())).await
    }

    /// Run the service under signal supervision and join its owned HTTP resources.
    ///
    /// The future lifecycle uses the existing five/six-second shutdown budget,
    /// not a test shutdown endpoint or a second HTTP runtime.
    ///
    /// # Errors
    /// Always refuses in T0; no listener, signal task or background task is started.
    pub async fn run(self) -> Result<(), DomainError> {
        std::future::ready(Err(pending())).await
    }
}

/// Typed configuration capture, preserving the existing setup builder pattern.
#[derive(Debug)]
pub struct AppBuilder {
    config: AppConfig,
}

impl AppBuilder {
    /// Capture configuration without validation, I/O or application construction.
    #[must_use]
    pub const fn new(config: AppConfig) -> Self {
        Self { config }
    }

    /// Prepare an application through the same startup entry as direct callers.
    ///
    /// # Errors
    /// Always refuses in T0 through `Application::new`.
    pub async fn build(self) -> Result<Application, DomainError> {
        Application::new(self.config).await
    }
}

fn pending() -> DomainError {
    DomainError::InvalidInput {
        message: "OAuthConnect service startup not implemented".to_owned(),
    }
}
