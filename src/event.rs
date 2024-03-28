use std::future::Future;

use json_elem::JsonElem;
use tokio::net::TcpStream;

use crate::{
    error::CommonErrors,
    message::{Event, MessageType, SocketMessage},
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
    util, RemoteError,
};

#[derive(Clone, Debug)]
pub struct EventListener {
    socket: Socket,
}

impl EventListener {
    pub async fn dispatch() -> Result<Self, RemoteError> {
        let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
        let stream = TcpStream::connect(server_address)
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        let addr = stream.peer_addr().unwrap();
        Ok(Self {
            socket: Socket::new(stream, addr),
        })
    }

    pub async fn listen<
        F: Future<Output = Result<(), RE>> + Send,
        RE: std::error::Error + 'static + Send,
        T: FnOnce(JsonElem) -> F + Send + Sync + Clone + 'static,
    >(
        &self,
        event_name: &str,
        callback: T,
    ) -> Result<(), RemoteError> {
        let msg = SocketMessage::new()
            .set_kind(MessageType::SubscribeEventRequest)
            .set_body(event_name.as_bytes());
        let stream = serde_json::to_vec(&msg)
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;
        self.socket
            .write(stream.as_slice())
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        let socket = self.socket.clone();

        tokio::spawn(async move {
            loop {
                let mut buf = Vec::new();
                let n = socket
                    .read(&mut buf)
                    .await
                    .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))
                    .unwrap_or_else(|e| {
                        log::error!("{:?}", e);
                        0
                    });
                if n == 0 {
                    log::error!(
                        "listen Error: {}",
                        CommonErrors::ServerConnectionError.to_string()
                    );
                    break;
                }
                let sep = util::separate(buf.as_slice());
                for data in sep {
                    if let Ok(msg) = serde_json::from_slice::<SocketMessage>(data.as_slice()) {
                        let param = serde_json::from_slice::<Event>(msg.body()).unwrap();
                        let call = callback.clone();
                        match call(param.param).await {
                            Ok(_) => {
                                continue;
                            }
                            Err(err) => {
                                log::error!("callback error: {err:?}");
                                break;
                            }
                        }
                    } else {
                        log::error!(
                            " listenError: {}",
                            CommonErrors::SerdeParseError.to_string()
                        );
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}
