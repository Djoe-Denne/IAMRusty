use crate::common::{spawn_test_server, TestDatabase};
use infra::config::load_config;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use std::sync::Arc;
use tokio::task::JoinHandle;

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
            println!("🔄 Previous server has stopped, starting a new one...");
            *server_guard = None;
        }
        
        // First, ensure the test database is running
        println!("🗄️  Setting up test database...");
        let _test_db = TestDatabase::new().await.map_err(|e| {
            format!("Failed to setup test database: {}", e)
        })?;
        println!("✅ Test database is ready");
        
        // Start the server using the existing spawn_test_server function
        let server_handle = tokio::spawn(async {
            if let Err(e) = spawn_test_server().await {
                eprintln!("Server failed to start: {}", e);
            }
        });
        
        *server_guard = Some(server_handle);
        
        // Wait for the server to be ready by trying to connect
        let client = reqwest::Client::new();
        
        // Try to connect to the server with retries
        for attempt in 1..=10 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempt)).await;
            
            if let Ok(response) = client.get(&format!("{}/health", base_url)).send().await {
                if response.status().is_success() {
                    println!("✅ Server is ready after {} attempts", attempt);
                    break;
                }
            }
            
            if attempt == 10 {
                eprintln!("❌ Server failed to start after 10 attempts");
                return Err("Server failed to start".into());
            }
            
            println!("⏳ Waiting for server to start (attempt {}/10)...", attempt);
        }
    } else {
        println!("♻️  Reusing existing server instance");
    }
    
    // Return the base URL based on test config
    eprintln!("🔗 Test client will connect to: {}", base_url);
    Ok(base_url)
} 