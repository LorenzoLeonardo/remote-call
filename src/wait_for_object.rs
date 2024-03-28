use json_elem::jsonelem::JsonElem;
use tokio::net::TcpStream;

use crate::{
    error::Error,
    message::{MessageType, SocketMessage},
    objects::SUCCESS,
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
};

/// A function that will guarantees that the object is already available for
/// remote method calls for synchronization purposes.
pub async fn wait_for_objects(list: Vec<String>) -> Result<(), Error> {
    let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
    let stream = TcpStream::connect(server_address).await.unwrap();

    let addr = stream.peer_addr().unwrap();
    let socket = Socket::new(stream, addr);

    for body in list {
        loop {
            let request = SocketMessage::new()
                .set_kind(MessageType::WaitForObject)
                .set_body(body.as_bytes());
            let stream = serde_json::to_vec(&request)
                .map_err(|err| Error::new(JsonElem::String(err.to_string())))?;

            socket
                .write(stream.as_slice())
                .await
                .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

            let mut buf = Vec::new();
            let n = socket
                .read(&mut buf)
                .await
                .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

            let reply = serde_json::from_slice::<SocketMessage>(&buf[0..n])
                .map_err(|e| Error::new(JsonElem::String(e.to_string())))?;

            if reply.body() == SUCCESS.as_bytes() {
                break;
            } else {
                tokio::task::yield_now().await;
                continue;
            }
        }
    }
    Ok(())
}
