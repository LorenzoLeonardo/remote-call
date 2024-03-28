use json_elem::JsonElem;
use tokio::net::TcpStream;

use crate::{
    error::{CommonErrors, RemoteError},
    message::{CallMethod, Event, MessageType, SocketMessage},
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
};

/// An object that is responsible for remote object method calls,
/// sending events and listening for incoming events.
#[derive(Clone, Debug)]
pub struct Connector {
    socket: Socket,
}

impl Connector {
    /// Connects to the IPC server.
    pub async fn connect() -> Result<Self, RemoteError> {
        let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
        let stream = TcpStream::connect(server_address)
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;

        let addr = stream.peer_addr().unwrap();
        Ok(Self {
            socket: Socket::new(stream, addr),
        })
    }

    /// Calls shared object methods from other processes.
    /// It has an optional parameters, the value is in JsonElem type.
    pub async fn remote_call(
        &self,
        object: &str,
        method: &str,
        param: JsonElem,
    ) -> Result<JsonElem, RemoteError> {
        let call_method = CallMethod {
            object: object.to_string(),
            method: method.to_string(),
            param,
        };

        let msg = SocketMessage::new()
            .set_kind(MessageType::RemoteCallRequest)
            .set_body(&call_method.as_bytes());

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

        let resp = serde_json::from_slice::<SocketMessage>(&buf[0..n])
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;
        if resp.kind() == MessageType::RemoteCallResponse {
            let json = JsonElem::try_from(resp.body())
                .map_err(|err| RemoteError::new(JsonElem::String(err.to_string())))?;
            Ok(json)
        } else {
            Err(RemoteError::new(JsonElem::String(
                CommonErrors::InvalidResponseData.to_string(),
            )))
        }
    }

    /// Sends the event to the server and let the server
    /// boadcast the message to all subscribed processes.
    /// Parameters in JsonElem type.
    pub async fn send_event(&self, event: &str, param: JsonElem) -> Result<(), RemoteError> {
        let event = Event {
            event: event.to_string(),
            param,
        };

        let msg = SocketMessage::new()
            .set_kind(MessageType::SendEventRequest)
            .set_body(&event.as_bytes());

        self.socket
            .write(&msg.as_bytes())
            .await
            .map_err(|e| RemoteError::new(JsonElem::String(e.to_string())))?;
        Ok(())
    }
}
