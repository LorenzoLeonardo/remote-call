mod list;
mod socket;

use tokio::net::TcpListener;

use crate::{list::ClientList, socket::Socket};

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
    start_server().await;
}

async fn start_server() {
    let listener = TcpListener::bind("127.0.0.1:1986").await.unwrap();

    log::trace!("Server listening on {}", "127.0.0.1:1986");
    let list_client = ClientList::new();

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let socket = Socket::new(socket, addr);

        let mut list = list_client.clone();
        tokio::spawn(async move {
            log::trace!("Connected: {}", socket.ip_address());

            list.add(socket.ip_address(), socket.clone()).await;
            loop {
                let mut data = Vec::new();

                match socket.read(&mut data).await {
                    Ok(usize) => {
                        if usize == 0 {
                            break;
                        }
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                        break;
                    }
                }

                log::trace!("Result Read: {:?}", data);
                broad_cast(&list, data.as_slice(), &socket.ip_address()).await;
            }
            log::trace!("Disconnected: {}", socket.ip_address());

            list.remove(&socket.ip_address()).await;
        });
    }
}

async fn broad_cast(list: &ClientList, data: &[u8], sender: &str) {
    let list = list.iter().await;
    for (ip, socket) in list.iter() {
        if ip != sender {
            let res = socket.write(data).await;

            log::info!("Broadcast to {}: {:?}", ip, res);
        }
    }
}
