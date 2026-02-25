#[tokio::main]
async fn main() {
    let result = neobrew::run().await;

    proc_exit::exit(result);
}
