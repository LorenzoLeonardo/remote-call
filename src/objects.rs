use std::collections::HashMap;

use async_trait::async_trait;
use atticus::Actor;
use json_elem::jsonelem::JsonElem;
use serde::{Deserialize, Serialize};

use crate::{
    message::{MessageType, SocketMessage},
    socket::Socket,
};

pub struct ListObjects {
    objects: HashMap<String, Socket>,
    events: HashMap<String, Socket>,
}

pub enum RequestListObjects {
    Add(SocketMessage, Socket),
    Remove(SocketMessage),
    CallMethod(SocketMessage),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CallMethod {
    pub object: String,
    pub method: String,
    pub param: JsonElem,
}

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
                msg.set_body("success".as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
            Err(err) => {
                log::error!("ListObjects::add(): {:?}", err);
                msg.set_body("failed".as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
        }
    }

    pub fn remove(&mut self, msg: SocketMessage) -> SocketMessage {
        match String::from_utf8(msg.body().into()) {
            Ok(object) => {
                self.objects.remove(&object);
                msg.set_body("success".as_bytes())
                    .set_kind(MessageType::RemoveShareObjectResponse)
            }
            Err(err) => {
                log::error!("ListObjects::remove(): {:?}", err);
                msg.set_body("failed".as_bytes())
                    .set_kind(MessageType::RemoveShareObjectResponse)
            }
        }
    }

    pub async fn call_method(&mut self, msg: SocketMessage) -> SocketMessage {
        match serde_json::from_slice::<CallMethod>(msg.body()) {
            Ok(call_method) => {
                if let Some(remote) = self.objects.get(&call_method.object) {
                    match serde_json::to_vec(&msg) {
                        Ok(data) => match remote.write(&data).await {
                            Ok(_res) => msg
                                .set_body("success".as_bytes())
                                .set_kind(MessageType::RemoteCallResponse),
                            Err(err) => {
                                log::error!("ListObjects::call_method: {:?}", err);
                                msg.set_body("remote connection error".as_bytes())
                                    .set_kind(MessageType::RemoteCallResponse)
                            }
                        },
                        Err(_) => todo!(),
                    }
                } else {
                    msg.set_body("object not found".as_bytes())
                        .set_kind(MessageType::RemoteCallResponse)
                }
            }
            Err(err) => {
                log::error!("ListObjects::call_method(): {:?}", err);
                msg.set_body("failed".as_bytes())
                    .set_kind(MessageType::AddShareObjectResponse)
            }
        }
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CallMethod;
    use json_elem::jsonelem::JsonElem;

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
