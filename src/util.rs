pub fn separate(data: &[u8]) -> Vec<Vec<u8>> {
    let json_str = String::from_utf8(data.to_vec()).unwrap();

    if !json_str.contains("}{") {
        return vec![data.to_vec()];
    }
    let parts: Vec<String> = json_str
        .trim()
        .split("}{")
        .map(|part| part.to_string() + "}")
        .collect();

    let num_parts = parts.len();
    let mut ret = Vec::new();

    for (n, mut part) in parts.into_iter().enumerate() {
        if n >= 1 {
            part = "{".to_string() + &part;
        }
        if n == (num_parts - 1) {
            part.pop();
        }
        ret.push(part.as_bytes().to_vec());
    }
    ret
}

#[cfg(test)]
mod test {
    use crate::{message::SocketMessage, util};

    #[test]
    fn test_separate() {
        let json_str = r#"{"id":5,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}{"id":6,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}{"id":8,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}"#.as_bytes();
        let ret = util::separate(json_str);

        assert_eq!(ret.len(), 3);
        for data in ret {
            let _msg = serde_json::from_slice::<SocketMessage>(data.as_slice()).unwrap();
        }

        let json_str = r#"{"id":5,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}"#.as_bytes();
        let ret = util::separate(json_str);

        assert_eq!(ret.len(), 1);
        for data in ret {
            let _msg = serde_json::from_slice::<SocketMessage>(data.as_slice()).unwrap();
        }

        let json_str = r#"{"id":5,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}{"id":8,"kind":3,"msg":[34,84,104,105,115,32,105,115,32,109,121,32,114,101,115,112,111,110,115,101,32,102,114,111,109,32,109,97,110,103,111,34]}"#.as_bytes();
        let ret = util::separate(json_str);

        assert_eq!(ret.len(), 2);
        for data in ret {
            let _msg = serde_json::from_slice::<SocketMessage>(data.as_slice()).unwrap();
        }
    }
}
