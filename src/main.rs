use remote_call::{logger::setup_logger, server::start_server};

#[tokio::main(flavor = "current_thread")]

async fn main() {
    setup_logger();
    start_server().await;
}
