mod config;
mod routes;
mod runtime;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    runtime::run().await
}
