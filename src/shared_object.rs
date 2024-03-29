use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use json_elem::JsonElem;
use tokio::{net::TcpStream, sync::Mutex, task::JoinHandle};

use crate::{
    error::{CommonErrors, Error, RemoteError},
    message::{CallMethod, MessageType, SocketMessage},
    objects::FAILED,
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
    util,
};

#[async_trait]
pub trait SharedObject: Send + Sync + 'static {
    async fn remote_call(&self, method: &str, param: JsonElem) -> Result<JsonElem, RemoteError>;
}

type ListSharedObjects = Arc<Mutex<HashMap<String, Box<dyn SharedObject>>>>;
/// An object that is responsible in registering the object to the IPC server,
/// and spawning a tokio task to handling incoming remote method calls from
/// other processes.
pub struct SharedObjectDispatcher {
    socket: Socket,
    list: ListSharedObjects,
}

impl SharedObjectDispatcher {
    /// Create a new ObjectDispatcher object and connects to the IPC server.
    pub async fn new() -> Result<Self, RemoteError> {
        let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
        let stream = TcpStream::connect(server_address)
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

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
    ) -> Result<(), RemoteError> {
        let mut list = self.list.lock().await;

        list.insert(object.to_string(), shared_object);

        let msg = SocketMessage::new()
            .set_kind(MessageType::AddShareObjectRequest)
            .set_body(object.as_bytes());

        self.socket
            .write(&msg.as_bytes())
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        let mut buf = Vec::new();
        let n = self
            .socket
            .read(&mut buf)
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        let msg = serde_json::from_slice::<SocketMessage>(&buf[0..n])
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        if msg.kind() == MessageType::AddShareObjectResponse {
            if msg.body() == FAILED.as_bytes() {
                panic!("Registering of {} failed!", object);
            }
        } else {
            panic!("Invalid message from server. {:?}", msg);
        }
        Ok(())
    }

    /// This handles remote object method call from other processess.
    /// It spawns a tokio task to handle the calls asynchronously and sends
    /// back the response back to the remote process.
    pub async fn spawn(&mut self) -> JoinHandle<Result<(), Error>> {
        let socket = self.socket.clone();
        let list = self.list.clone();

        tokio::spawn(async move {
            loop {
                let mut buf = Vec::new();
                socket.read(&mut buf).await?;

                let sep = util::separate(buf.as_slice());
                for data in sep {
                    if let Ok(msg) = serde_json::from_slice::<SocketMessage>(data.as_slice()) {
                        if msg.kind() == MessageType::RemoteCallRequest {
                            Self::handle_remote_call_request(list.clone(), msg, socket.clone())
                                .await?;
                        }
                    } else {
                        log::error!("Invalid stream");
                        let mut msg = SocketMessage::new();
                        let err = RemoteError::new(JsonElem::String(
                            CommonErrors::SerdeParseError.to_string(),
                        ));
                        msg = msg
                            .set_body(&err.as_bytes())
                            .set_kind(MessageType::RemoteCallResponse);

                        socket.write(&msg.as_bytes()).await?;
                    }
                }
            }
        })
    }

    async fn handle_remote_call_request(
        list: ListSharedObjects,
        mut msg: SocketMessage,
        socket: Socket,
    ) -> Result<(), Error> {
        let val = list.lock().await;
        if let Ok(call) = serde_json::from_slice::<CallMethod>(msg.body()) {
            let msg = if let Some(rem_call) = val.get(&call.object) {
                match rem_call.remote_call(&call.method, call.param).await {
                    Ok(response) => {
                        let body: Vec<u8> = response.try_into()?;
                        msg = msg
                            .set_body(&body)
                            .set_kind(MessageType::RemoteCallResponse);
                        msg
                    }
                    Err(err) => {
                        let response = JsonElem::convert_from(&err)?;
                        let body: Vec<u8> = response.try_into()?;
                        msg = msg
                            .set_body(&body)
                            .set_kind(MessageType::RemoteCallResponse);
                        msg
                    }
                }
            } else {
                let err =
                    RemoteError::new(JsonElem::String(CommonErrors::ObjectNotFound.to_string()));
                msg = msg
                    .set_body(&err.as_bytes())
                    .set_kind(MessageType::RemoteCallResponse);
                msg
            };
            socket.write(&msg.as_bytes()).await?;
        } else {
            let err = RemoteError::new(JsonElem::String(CommonErrors::SerdeParseError.to_string()));
            msg = msg
                .set_body(&err.as_bytes())
                .set_kind(MessageType::RemoteCallResponse);

            socket.write(&msg.as_bytes()).await?;
        }
        Ok(())
    }
}
