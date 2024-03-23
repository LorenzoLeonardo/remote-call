use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MessageType {
    ShareObject,
    RemoteCall,
    SendEvent,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::ShareObject
    }
}

impl Serialize for MessageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value_str = match self {
            MessageType::ShareObject => 0,
            MessageType::RemoteCall => 1,
            MessageType::SendEvent => 2,
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
            0 => Ok(MessageType::ShareObject),
            1 => Ok(MessageType::RemoteCall),
            2 => Ok(MessageType::SendEvent),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid value for MessageType(0,1,2): {}",
                value
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct SocketMessage {
    id: u32,
    kind: MessageType,
    msg: Vec<u8>,
}

impl SocketMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_id(mut self, id: u32) -> Self {
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

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn kind(&self) -> MessageType {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{MessageType, SocketMessage};

    #[test_case(0, MessageType::ShareObject, "hello world".as_bytes(),
        r#"{"id":0,"kind":0,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#; "ShareObject")]
    #[test_case(0, MessageType::RemoteCall, "hello world".as_bytes(),
        r#"{"id":0,"kind":1,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#; "RemoteCall")]
    #[test_case(0, MessageType::SendEvent, "hello world".as_bytes(),
        r#"{"id":0,"kind":2,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#; "SendEvent")]
    fn sock_message_serialize(id: u32, kind: MessageType, body: &[u8], expected: &str) {
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
        .set_kind(MessageType::ShareObject);
    "ShareObject")]
    #[test_case( r#"{"id":0,"kind":1,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#.as_bytes(),
        SocketMessage::new()
        .set_body(&[104,101,108,108,111,32,119,111,114,108,100])
        .set_id(0)
        .set_kind(MessageType::RemoteCall);
    "RemoteCall")]
    #[test_case( r#"{"id":0,"kind":2,"msg":[104,101,108,108,111,32,119,111,114,108,100]}"#.as_bytes(),
        SocketMessage::new()
        .set_body(&[104,101,108,108,111,32,119,111,114,108,100])
        .set_id(0)
        .set_kind(MessageType::SendEvent);
    "SendEvent")]
    fn sock_message_deserialize(stream: &[u8], expected: SocketMessage) {
        let msg: SocketMessage = serde_json::from_slice(stream).unwrap();

        assert_eq!(msg, expected);
    }
}
