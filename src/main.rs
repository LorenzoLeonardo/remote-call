use remote_call::{logger::setup_logger, server::start_server};

#[ctor::ctor]
fn set_log() {
    setup_logger();
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let version = env!("CARGO_PKG_VERSION");

    log::info!("Starting remote-call v.{}", version);
    start_server().await;
    log::info!("Ending remote-call v.{}", version);
}
