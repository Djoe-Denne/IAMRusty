//! Existing legacy names belong to their containers, not to a new fixture.
//! The canaries are created by this test through the ownership-aware harness.

#[path = "fixtures/mod.rs"]
mod fixtures;

use std::time::Duration;

use rustycog::testing::common::{start_service_testcontainer, OwnedTestContainer};
use rustycog::testing::testcontainers::{
    self,
    core::{ContainerPort::Tcp, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};
use serial_test::serial;

const KEY: &str = "fixture-name-canary";

struct Canary {
    _container: OwnedTestContainer,
    name: &'static str,
    id: String,
    client: redis::Client,
    port: u16,
}

impl Canary {
    async fn start(name: &'static str) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        let image = GenericImage::new("redis", "7-alpine")
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
            .with_container_name(name)
            .with_mapped_port(port, Tcp(6379));
        let container =
            start_service_testcontainer(image.start(), "name-canary", name, port).await?;
        let client = redis::Client::open(format!("redis://127.0.0.1:{port}"))?;
        let mut connection = client.get_connection_with_timeout(Duration::from_secs(5))?;
        connection.set_read_timeout(Some(Duration::from_secs(5)))?;
        redis::cmd("SET")
            .arg(KEY)
            .arg(name)
            .query::<()>(&mut connection)?;
        container.ready()?;
        // Inspection is only an assertion witness. Cleanup uses the owned handle
        // and parent creation ledger, never this discovered ID or a name.
        let id = running_identity(name).await?;
        Ok(Self {
            _container: container,
            name,
            id,
            client,
            port,
        })
    }

    async fn assert_unchanged(&self) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(running_identity(self.name).await?, self.id);
        let mut connection = self
            .client
            .get_connection_with_timeout(Duration::from_secs(5))?;
        connection.set_read_timeout(Some(Duration::from_secs(5)))?;
        let value: String = redis::cmd("GET").arg(KEY).query(&mut connection)?;
        assert_eq!(value, self.name);
        Ok(())
    }
}

async fn running_identity(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let docker = testcontainers::core::client::docker_client_instance().await?;
    let inspect = docker.inspect_container(name, None).await?;
    assert_eq!(inspect.name.as_deref(), Some(format!("/{name}").as_str()));
    assert_eq!(
        inspect.state.as_ref().and_then(|state| state.running),
        Some(true)
    );
    Ok(inspect.id.ok_or("canary has no inspect ID")?)
}

#[tokio::test]
#[serial]
async fn fixture_start_preserves_legacy_name_canaries() {
    let openbao_canary = Canary::start("lazaret_test-openbao")
        .await
        .expect("create owned OpenBao-name canary");
    let redis_canary = Canary::start("lazaret_test-redis")
        .await
        .expect("create owned Redis-name canary");

    let openbao = fixtures::TestOpenBao::new()
        .await
        .expect("new OpenBao must not evict the old-name canary");
    let redis = fixtures::TestRedis::new()
        .await
        .expect("new Redis must not reuse the old-name canary");
    assert_ne!(openbao.port, openbao_canary.port);
    assert_ne!(redis.port, redis_canary.port);
    openbao
        .put_kv_v2("fixture-isolation/probe", "value", "fixture-payload")
        .await
        .expect("new OpenBao remains usable");
    redis.flush().expect("flush only the new Redis");

    openbao_canary
        .assert_unchanged()
        .await
        .expect("OpenBao-name canary identity, running state and data preserved");
    redis_canary
        .assert_unchanged()
        .await
        .expect("Redis-name canary identity, running state and data preserved");
}
