mod message;
mod objects;
mod socket;

use std::{collections::HashMap, sync::Arc};

use atticus::run_actor;
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    message::{MessageType, SocketMessage},
    objects::{ListObjects, RequestListObjects},
    socket::Socket,
};

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
    let list_call_object = Arc::new(Mutex::new(HashMap::<u32, Socket>::new()));

    let res = run_actor(ListObjects::new(), 2);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let socket = Socket::new(socket, addr);
        let list_object_requestor = res.requestor.clone();
        let inner_list_call_object = list_call_object.clone();
        tokio::spawn(async move {
            log::trace!("Connected: {}", socket.ip_address());

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

                match serde_json::from_slice::<SocketMessage>(data.as_slice()) {
                    Ok(mut msg) => match msg.kind() {
                        MessageType::AddShareObjectRequest => {
                            log::info!("AddShareObjectRequest: {:?}", msg);
                            let _ = list_object_requestor
                                .request(RequestListObjects::Add(msg, socket.clone()))
                                .await;
                        }
                        MessageType::AddShareObjectResponse => {
                            log::info!("AddShareObjectResponse: {:?}", msg);
                        }
                        MessageType::RemoteCallRequest => {
                            log::info!("RemoteCallRequest: {:?}", msg);
                            let mut lst = inner_list_call_object.lock().await;
                            let id = lst.len() as u32;
                            msg = msg.set_id(id);
                            lst.insert(id, socket.clone());

                            let _ = list_object_requestor
                                .request(RequestListObjects::CallMethod(msg))
                                .await;
                        }
                        MessageType::RemoteCallResponse => {
                            log::info!("RemoteCallResponse: {:?}", msg);

                            let mut lst = inner_list_call_object.lock().await;
                            if let Some(remote) = lst.get(&msg.id()) {
                                match serde_json::to_vec(&msg) {
                                    Ok(data) => match remote.write(&data).await {
                                        Ok(_res) => {
                                            lst.remove(&msg.id());
                                        }
                                        Err(err) => {
                                            log::error!("{:?}", err);
                                        }
                                    },
                                    Err(_) => todo!(),
                                }
                            }
                        }
                        MessageType::SendEventRequest => {
                            log::info!("SendEventRequest: {:?}", msg);
                        }
                        MessageType::SendEventResponse => {
                            log::info!("SendEventResponse: {:?}", msg);
                        }
                        MessageType::RegisterEventRequest => {
                            log::info!("RegisterEventRequest: {:?}", msg);
                        }
                        MessageType::RegisterEventResponse => {
                            log::info!("RegisterEventResponse: {:?}", msg);
                        }
                        MessageType::RemoveShareObjectRequest => {
                            log::info!("RemoveShareObjectRequest: {:?}", msg);
                        }
                        MessageType::RemoveShareObjectResponse => {
                            log::info!("RemoveShareObjectResponse: {:?}", msg);
                        }
                    },
                    Err(error) => {
                        log::error!(
                            "Invalid message from {}: {}",
                            socket.ip_address(),
                            error.to_string()
                        )
                    }
                }
            }
            log::trace!("Disconnected: {}", socket.ip_address());
            let _ = list_object_requestor
                .request(RequestListObjects::Remove(socket.clone()))
                .await;
        });
    }
}
