#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Heap-allocate the boot future: `run` exceeds clippy's large_futures limit.
    Box::pin(oodhive_monolith::runtime::run()).await
}
