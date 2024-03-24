use json_elem::jsonelem::JsonElem;
use tokio::net::TcpStream;

use crate::{
    error::{CommonErrors, Error},
    message::{CallMethod, MessageType, SocketMessage},
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
    pub async fn connect() -> Result<Self, Error> {
        let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
        let stream = TcpStream::connect(server_address)
            .await
            .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

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
    ) -> Result<JsonElem, Error> {
        let call_method = CallMethod {
            object: object.to_string(),
            method: method.to_string(),
            param,
        };
        let body = serde_json::to_vec(&call_method)
            .map_err(|err| Error::new(JsonElem::String(err.to_string())))?;
        let request = SocketMessage::new()
            .set_kind(MessageType::RemoteCallRequest)
            .set_body(&body);

        let stream = serde_json::to_vec(&request)
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

        if n == 0 {
            Err(Error::new(JsonElem::String(
                CommonErrors::RemoteConnectionError.to_string(),
            )))
        } else {
            let resp = serde_json::from_slice::<SocketMessage>(&buf[0..n])
                .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;
            if resp.kind() == MessageType::RemoteCallResponse {
                let json = JsonElem::try_from(resp.body())
                    .map_err(|err| Error::new(JsonElem::String(err.to_string())))?;
                Ok(json)
            } else {
                Err(Error::new(JsonElem::String(
                    CommonErrors::InvalidResponseData.to_string(),
                )))
            }
        }
    }
}
