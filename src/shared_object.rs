use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use json_elem::jsonelem::JsonElem;
use tokio::{net::TcpStream, sync::Mutex, task::JoinHandle};

use crate::{
    error::{CommonErrors, Error},
    message::{CallMethod, MessageType, SocketMessage},
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
};

#[async_trait]
pub trait SharedObject: Send + Sync + 'static {
    async fn remote_call(&self, method: &str, param: JsonElem) -> Result<JsonElem, Error>;
}

/// An object that is responsible in registering the object to the IPC server,
/// and spawning a tokio task to handling incoming remote method calls from
/// other processes.
pub struct SharedObjectDispatcher {
    socket: Socket,
    list: Arc<Mutex<HashMap<String, Box<dyn SharedObject>>>>,
}

impl SharedObjectDispatcher {
    /// Create a new ObjectDispatcher object and connects to the IPC server.
    pub async fn new() -> Result<Self, Error> {
        let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
        let stream = TcpStream::connect(server_address)
            .await
            .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

        let addr = stream.peer_addr().unwrap();
        Ok(Self {
            socket: Socket::new(stream, addr),
            list: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    /// This registers the Shared Object into the IPC server.
    pub async fn register_object(
        &mut self,
        object: &str,
        shared_object: Box<dyn SharedObject>,
    ) -> Result<(), Error> {
        let mut list = self.list.lock().await;

        list.insert(object.to_string(), shared_object);

        let object = SocketMessage::new()
            .set_kind(MessageType::AddShareObjectRequest)
            .set_body(object.as_bytes());
        let stream = serde_json::to_vec(&object)
            .map_err(|err| Error::new(JsonElem::String(err.to_string())))?;

        self.socket
            .write(stream.as_slice())
            .await
            .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

        let mut buf = Vec::new();
        let n = self
            .socket
            .read(&mut buf)
            .await
            .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

        let _ = serde_json::from_slice::<SocketMessage>(&buf[0..n])
            .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;
        Ok(())
    }

    /// This handles remote object method call from other processess.
    /// It spawns a tokio task to handle the calls asynchronously and sends
    /// back the response back to the remote process.
    pub async fn spawn(&mut self) -> JoinHandle<()> {
        let socket = self.socket.clone();
        let list = self.list.clone();
        tokio::spawn(async move {
            loop {
                let mut buf = Vec::new();
                let n = socket.read(&mut buf).await.map_or_else(
                    |e| {
                        log::error!("{:?}", e);
                        0
                    },
                    |size: usize| size,
                );

                if n == 0 {
                    log::error!("Error: {}", CommonErrors::ServerConnectionError.to_string());
                    break;
                } else if let Ok(mut msg) = serde_json::from_slice::<SocketMessage>(&buf[0..n]) {
                    if msg.kind() == MessageType::RemoteCallRequest {
                        let val = list.lock().await;
                        if let Ok(call) = serde_json::from_slice::<CallMethod>(msg.body()) {
                            let response = if let Some(rem_call) = val.get(&call.object) {
                                match rem_call.remote_call(&call.method, call.param).await {
                                    Ok(response) => {
                                        let body: Vec<u8> = response.try_into().unwrap();
                                        msg = msg
                                            .set_body(&body)
                                            .set_kind(MessageType::RemoteCallResponse);
                                        msg
                                    }
                                    Err(err) => {
                                        let response = JsonElem::convert_from(&err).unwrap();
                                        let body: Vec<u8> = response.try_into().unwrap();
                                        msg = msg
                                            .set_body(&body)
                                            .set_kind(MessageType::RemoteCallResponse);
                                        msg
                                    }
                                }
                            } else {
                                let err = Error::new(JsonElem::String(
                                    CommonErrors::ObjectNotFound.to_string(),
                                ));
                                let response = JsonElem::convert_from(&err).unwrap();
                                let body: Vec<u8> = response.try_into().unwrap();
                                msg = msg
                                    .set_body(&body)
                                    .set_kind(MessageType::RemoteCallResponse);
                                msg
                            };
                            let stream = serde_json::to_vec(&response)
                                .map_err(|err| Error::new(JsonElem::String(err.to_string())))
                                .unwrap();

                            if socket.write(stream.as_slice()).await.is_err() {
                                break;
                            }
                        } else {
                            let err = Error::new(JsonElem::String(
                                CommonErrors::SerdeParseError.to_string(),
                            ));
                            let response = JsonElem::convert_from(&err).unwrap();
                            let body: Vec<u8> = response.try_into().unwrap();
                            msg = msg
                                .set_body(&body)
                                .set_kind(MessageType::RemoteCallResponse);

                            let stream = serde_json::to_vec(&msg)
                                .map_err(|err| Error::new(JsonElem::String(err.to_string())))
                                .unwrap();
                            if socket.write(stream.as_slice()).await.is_err() {
                                break;
                            }
                        }
                    }
                } else {
                    let mut msg = SocketMessage::new();
                    let err =
                        Error::new(JsonElem::String(CommonErrors::SerdeParseError.to_string()));
                    let response = JsonElem::convert_from(&err).unwrap();
                    let body: Vec<u8> = response.try_into().unwrap();
                    msg = msg
                        .set_body(&body)
                        .set_kind(MessageType::RemoteCallResponse);

                    let stream = serde_json::to_vec(&msg)
                        .map_err(|err| Error::new(JsonElem::String(err.to_string())))
                        .unwrap();
                    if socket.write(stream.as_slice()).await.is_err() {
                        break;
                    }
                }
            }
        })
    }
}
