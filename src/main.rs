#[tokio::main]
async fn main() {
    let exit_result = neobrew::run().await;

    proc_exit::exit(exit_result);
}
