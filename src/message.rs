use std::fmt::Display;

use json_elem::JsonElem;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum MessageType {
    #[default]
    AddShareObjectRequest,
    AddShareObjectResponse,
    RemoteCallRequest,
    RemoteCallResponse,
    SendEventRequest,
    SendEventResponse,
    SubscribeEventRequest,
    SubscribeEventResponse,
    RemoveShareObjectRequest,
    RemoveShareObjectResponse,
    WaitForObject,
}

impl Serialize for MessageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value_str = match self {
            MessageType::AddShareObjectRequest => 0,
            MessageType::AddShareObjectResponse => 1,
            MessageType::RemoteCallRequest => 2,
            MessageType::RemoteCallResponse => 3,
            MessageType::SendEventRequest => 4,
            MessageType::SendEventResponse => 5,
            MessageType::SubscribeEventRequest => 6,
            MessageType::SubscribeEventResponse => 7,
            MessageType::RemoveShareObjectRequest => 8,
            MessageType::RemoveShareObjectResponse => 9,
            MessageType::WaitForObject => 10,
        };
        serializer.serialize_u32(value_str)
    }
}

impl<'de> Deserialize<'de> for MessageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: u32 = Deserialize::deserialize(deserializer)?;
        match value {
            0 => Ok(MessageType::AddShareObjectRequest),
            1 => Ok(MessageType::AddShareObjectResponse),
            2 => Ok(MessageType::RemoteCallRequest),
            3 => Ok(MessageType::RemoteCallResponse),
            4 => Ok(MessageType::SendEventRequest),
            5 => Ok(MessageType::SendEventResponse),
            6 => Ok(MessageType::SubscribeEventRequest),
            7 => Ok(MessageType::SubscribeEventResponse),
            8 => Ok(MessageType::RemoveShareObjectRequest),
            9 => Ok(MessageType::RemoveShareObjectResponse),
            10 => Ok(MessageType::WaitForObject),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid value for MessageType(0,1,2,3,4,5,6,7,8,9,10): {}",
                value
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct SocketMessage {
    id: u64,
    kind: MessageType,
    msg: Vec<u8>,
}

impl SocketMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    pub fn set_kind(mut self, kind: MessageType) -> Self {
        self.kind = kind;
        self
    }

    pub fn set_body(mut self, body: &[u8]) -> Self {
        self.msg = body.to_owned();
        self
    }

    pub fn body(&self) -> &[u8] {
        &self.msg
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn kind(&self) -> MessageType {
        self.kind
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

impl Display for SocketMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} id: {} ", self.kind, self.id)
    }
}
pub fn result_to_socket_message(
    res: Result<Option<SocketMessage>, atticus::Error>,
    msg_type: MessageType,
) -> SocketMessage {
    match res {
        Ok(respon) => respon.unwrap(),
        Err(err) => SocketMessage::new()
            .set_kind(msg_type)
            .set_body(err.to_string().as_bytes()),
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CallMethod {
    pub object: String,
    pub method: String,
    pub param: JsonElem,
}

impl CallMethod {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Event {
    pub event: String,
    pub param: JsonElem,
}

impl Event {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use json_elem::JsonElem;
    use test_case::test_case;

    use crate::message::CallMethod;

    use super::{MessageType, SocketMessage};

    #[test_case(0, MessageType::AddShareObjectRequest, "my_object".as_bytes(),
        r#"{"id":0,"kind":0,"msg":[109,121,95,111,98,106,101,99,116]}"#; "ShareObject")]
    #[test_case(0, MessageType::RemoteCallRequest,
        &serde_json::to_vec(&CallMethod {
            object: "my_object".to_string(),
            method: "my_function".to_string(),
            param: JsonElem::String("test param".to_string()),
        }).unwrap(),
        r#"{"id":0,"kind":2,"msg":[123,34,111,98,106,101,99,116,34,58,34,109,121,95,111,98,106,101,99,116,34,44,34,109,101,116,104,111,100,34,58,34,109,121,95,102,117,110,99,116,105,111,110,34,44,34,112,97,114,97,109,34,58,34,116,101,115,116,32,112,97,114,97,109,34,125]}"#; "RemoteCall")]
    #[test_case(0, MessageType::SendEventRequest, "hello world".as_bytes(),
        r#"{"id":0,"kind":4,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#; "SendEvent")]
    fn sock_message_serialize(id: u64, kind: MessageType, body: &[u8], expected: &str) {
        let message = SocketMessage::new()
            .set_body(body)
            .set_id(id)
            .set_kind(kind);
        let msg = serde_json::to_vec(&message).unwrap();

        assert_eq!(msg.as_slice(), expected.as_bytes());
    }

    #[test_case( r#"{"id":0,"kind":0,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#.as_bytes(),
        SocketMessage::new()
        .set_body(&[104,101,108,108,111,32,119,111,114,108,100])
        .set_id(0)
        .set_kind(MessageType::AddShareObjectRequest);
    "ShareObject")]
    #[test_case( r#"{"id":0,"kind":2,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#.as_bytes(),
        SocketMessage::new()
        .set_body(&[104,101,108,108,111,32,119,111,114,108,100])
        .set_id(0)
        .set_kind(MessageType::RemoteCallRequest);
    "RemoteCall")]
    #[test_case( r#"{"id":0,"kind":4,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#.as_bytes(),
        SocketMessage::new()
        .set_body(&[104,101,108,108,111,32,119,111,114,108,100])
        .set_id(0)
        .set_kind(MessageType::SendEventRequest);
    "SendEvent")]
    fn sock_message_deserialize(stream: &[u8], expected: SocketMessage) {
        let msg: SocketMessage = serde_json::from_slice(stream).unwrap();

        assert_eq!(msg, expected);
    }
}
