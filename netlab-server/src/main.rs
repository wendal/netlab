use netlab_server::bootstrap::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}
