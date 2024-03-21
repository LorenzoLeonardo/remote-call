mod socket;

use tokio::net::TcpListener;

use crate::socket::Socket;

pub const ENV_LOGGER: &str = "RUST_LOG";

pub fn setup_logger() {
    let level = std::env::var(ENV_LOGGER)
        .map(|var| match var.to_lowercase().as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            "off" => log::LevelFilter::Off,
            _ => log::LevelFilter::Info,
        })
        .unwrap_or_else(|_| log::LevelFilter::Info);

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}:{}]: {}",
                chrono::Local::now().format("%H:%M:%S%.9f"),
                record.level(),
                record.target(),
                record.line().unwrap_or(0),
                message
            ))
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .unwrap_or_else(|e| {
            panic!("{:?}", e);
        });
}

#[tokio::main(flavor = "current_thread")]

async fn main() {
    setup_logger();
    let listener = TcpListener::bind("127.0.0.1:1986").await.unwrap();

    log::trace!("Server listening on {}", "127.0.0.1:1986");
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let socket = Socket::new(socket, addr);
        tokio::spawn(async move {
            log::trace!("Connected: {}", socket.ip_address());
            loop {
                let mut data = Vec::new();

                if (socket.read(&mut data).await).is_err() {
                    break;
                }
                log::trace!("Result Read: {:?}", data);

                if (socket.write("Hoy".as_bytes()).await).is_err() {
                    break;
                } else {
                    log::trace!("Result Send: Ok");
                }
            }
            log::trace!("Disconnected: {}", socket.ip_address());
        });
    }
}
