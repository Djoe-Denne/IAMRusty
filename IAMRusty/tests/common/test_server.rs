use crate::common::{spawn_test_server, TestFixture};
use configuration::load_config;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::debug;
use reqwest::Client;

/// Global test server instance that starts only once
static TEST_SERVER: OnceLock<Arc<Mutex<Option<JoinHandle<()>>>>> = OnceLock::new();

/// Get or create the global test server instance
pub async fn get_test_server() -> Result<String, Box<dyn std::error::Error>> {
    let server_mutex = TEST_SERVER.get_or_init(|| {
        Arc::new(Mutex::new(None))
    });
    
    let mut server_guard = server_mutex.lock().await;
    
    // Check if we need to start a new server
    let needs_new_server = match server_guard.as_ref() {
        None => true, // No server handle exists
        Some(handle) => handle.is_finished(), // Server handle exists but task is finished
    };
    
    
    let config = load_config().expect("failed to load test config");
    let base_url = format!("http://{}:{}", config.server.host, config.server.port);

    if needs_new_server {
        // If the old handle is finished, clear it
        if server_guard.is_some() {
            debug!("🔄 Previous server has stopped, starting a new one...");
            *server_guard = None;
        }
        
        // Start the server using the existing spawn_test_server function
        let server_handle = tokio::spawn(async {
            if let Err(e) = spawn_test_server().await {
                debug!("Server failed to start: {}", e);
            }
        });
        
        *server_guard = Some(server_handle);
        
    } else {
        debug!("♻️  Reusing existing server instance");
    }
    
    // Return the base URL based on test config
    debug!("🔗 Test client will connect to: {}", base_url);
    Ok(base_url)
} 

// method that return a test fixture, base_url and client
pub async fn setup_test_server() -> Result<(TestFixture, String, Client), Box<dyn std::error::Error>> {
    let fixture = TestFixture::new().await?;
    let base_url = get_test_server().await?;
    let client = create_test_client();
    Ok((fixture, base_url, client))
}

pub fn create_test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to create HTTP client")
}