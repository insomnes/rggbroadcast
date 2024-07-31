use anyhow::Result as AnyResult;
use rgg::run_node;

// With 2 max threads
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> AnyResult<()> {
    if let Err(e) = run_node(4).await {
        eprintln!("Error: {}", e);
    }
    Ok(())
}
