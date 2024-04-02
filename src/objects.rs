use std::collections::HashMap;

use async_trait::async_trait;
use atticus::Actor;
use json_elem::JsonElem;
use serde::Serialize;

use crate::{
    error::CommonErrors,
    message::{CallMethod, Event, MessageType, SocketMessage},
    socket::Socket,
    RemoteError,
};

pub struct ListObjects {
    objects: HashMap<String, Socket>,
    events: HashMap<String, Socket>,
}

pub enum RequestListObjects {
    Add(SocketMessage, Socket),
    Remove(Socket),
    CallMethod(SocketMessage),
    WaitForObject(SocketMessage),
    SubscribeEvent(SocketMessage, Socket),
    SendEvent(SocketMessage),
    ListObject,
}

pub const SUCCESS: &str = "success";
pub const FAILED: &str = "failed";

impl ListObjects {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            events: HashMap::new(),
        }
    }

    pub fn add(&mut self, msg: SocketMessage, socket: Socket) -> SocketMessage {
        match String::from_utf8(msg.body().into()) {
            Ok(object) => {
                self.objects.insert(object, socket);
                msg.set_body(SUCCESS.as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
            Err(err) => {
                log::error!("ListObjects::add(): {:?}", err);
                msg.set_body(FAILED.as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
        }
    }

    pub fn remove(&mut self, socket: Socket) -> SocketMessage {
        self.objects
            .retain(|_key, value| value.ip_address() != socket.ip_address());

        self.events
            .retain(|_key, value| value.ip_address() != socket.ip_address());
        SocketMessage::new().set_kind(MessageType::RemoveShareObjectResponse)
    }

    pub async fn call_method(&mut self, msg: SocketMessage) -> SocketMessage {
        match serde_json::from_slice::<CallMethod>(msg.body()) {
            Ok(call_method) => {
                if let Some(remote) = self.objects.get(&call_method.object) {
                    match remote.write(&msg.as_bytes()).await {
                        Ok(_) => msg
                            .set_body(SUCCESS.as_bytes())
                            .set_kind(MessageType::RemoteCallResponse),
                        Err(err) => {
                            log::error!("ListObjects::call_method: {:?}", err);
                            let _ = self.remove(remote.clone());
                            msg.set_body("remote connection error".as_bytes())
                                .set_kind(MessageType::RemoteCallResponse)
                        }
                    }
                } else {
                    let err = RemoteError::new(JsonElem::String(
                        CommonErrors::ObjectNotFound.to_string(),
                    ));
                    msg.set_body(&err.as_bytes())
                        .set_kind(MessageType::RemoteCallResponse)
                }
            }
            Err(err) => {
                log::error!("ListObjects::call_method(): {:?}", err);
                let err =
                    RemoteError::new(JsonElem::String(CommonErrors::SerdeParseError.to_string()));
                msg.set_body(&err.as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
        }
    }

    pub async fn wait_for_object(&mut self, msg: SocketMessage) -> SocketMessage {
        match String::from_utf8(msg.body().into()) {
            Ok(object) => {
                if self.objects.get(object.as_str()).is_some() {
                    msg.set_body(SUCCESS.as_bytes())
                        .set_kind(MessageType::WaitForObject)
                } else {
                    msg.set_body(FAILED.as_bytes())
                        .set_kind(MessageType::WaitForObject)
                }
            }
            Err(err) => {
                log::error!("ListObjects::wait_for_object(): {:?}", err);
                msg.set_body(FAILED.as_bytes())
                    .set_kind(MessageType::WaitForObject)
            }
        }
    }

    pub fn subscribe_event(&mut self, msg: SocketMessage, socket: Socket) -> SocketMessage {
        match String::from_utf8(msg.body().into()) {
            Ok(event_name) => {
                self.events.insert(event_name, socket);
                msg.set_body(SUCCESS.as_bytes())
                    .set_kind(MessageType::SubscribeEventResponse)
            }
            Err(err) => {
                log::error!("ListObjects::subscribe_event(): {:?}", err);
                msg.set_body(FAILED.as_bytes())
                    .set_kind(MessageType::SubscribeEventResponse)
            }
        }
    }

    pub async fn send_event(&mut self, msg: SocketMessage) -> SocketMessage {
        match serde_json::from_slice::<Event>(msg.body()) {
            Ok(event) => {
                for (event_name, socket) in &self.events {
                    if *event_name == event.event {
                        let ret = socket.write(&msg.as_bytes()).await;
                        log::trace!("ListObjects::send_event: {:?}", ret);
                    }
                }
                msg.set_body(SUCCESS.as_bytes())
                    .set_kind(MessageType::SendEventResponse)
            }
            Err(err) => {
                log::error!("ListObjects::send_event(): {:?}", err);
                msg.set_body(FAILED.as_bytes())
                    .set_kind(MessageType::SendEventResponse)
            }
        }
    }

    pub fn list_objects(&self) -> SocketMessage {
        #[derive(Serialize)]
        struct Object {
            objects: Vec<String>,
        }
        let mut objects = Vec::new();

        for (key, _) in self.objects.iter() {
            objects.push(key.to_string());
        }
        let object = Object { objects };
        let slice =
            serde_json::to_vec(&object).unwrap_or_else(|err| err.to_string().as_bytes().to_vec());

        SocketMessage::new().set_body(slice.as_slice())
    }
}
#[async_trait]
impl Actor for ListObjects {
    type Request = RequestListObjects;
    type Response = SocketMessage;

    async fn handle(&mut self, message: Self::Request) -> Option<Self::Response> {
        match message {
            RequestListObjects::Add(msg, socket) => Some(self.add(msg, socket)),
            RequestListObjects::Remove(msg) => Some(self.remove(msg)),
            RequestListObjects::CallMethod(msg) => Some(self.call_method(msg).await),
            RequestListObjects::WaitForObject(msg) => Some(self.wait_for_object(msg).await),
            RequestListObjects::SubscribeEvent(msg, socket) => {
                Some(self.subscribe_event(msg, socket))
            }
            RequestListObjects::SendEvent(msg) => Some(self.send_event(msg).await),
            RequestListObjects::ListObject => Some(self.list_objects()),
        }
    }
}

impl Default for ListObjects {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::message::CallMethod;
    use json_elem::JsonElem;

    #[test]
    fn test_call_method() {
        let call = CallMethod {
            object: "my_object".to_string(),
            method: "my_function".to_string(),
            param: JsonElem::String("test param".to_string()),
        };

        assert_eq!(
            serde_json::to_vec(&call).unwrap(),
            &[
                123, 34, 111, 98, 106, 101, 99, 116, 34, 58, 34, 109, 121, 95, 111, 98, 106, 101,
                99, 116, 34, 44, 34, 109, 101, 116, 104, 111, 100, 34, 58, 34, 109, 121, 95, 102,
                117, 110, 99, 116, 105, 111, 110, 34, 44, 34, 112, 97, 114, 97, 109, 34, 58, 34,
                116, 101, 115, 116, 32, 112, 97, 114, 97, 109, 34, 125
            ]
        );
    }
}
