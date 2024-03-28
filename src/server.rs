use std::{collections::HashMap, sync::Arc};

use atticus::run_actor;
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    message::{self, MessageType, SocketMessage},
    objects::SUCCESS,
    socket::{Socket, ENV_SERVER_ADDRESS, SERVER_ADDRESS},
    util,
};

use crate::objects::{ListObjects, RequestListObjects};

pub async fn start_server() {
    let server_address = std::env::var(ENV_SERVER_ADDRESS).unwrap_or(SERVER_ADDRESS.to_owned());
    let listener = TcpListener::bind(server_address.as_str()).await.unwrap();

    log::trace!("Server listening on {}", server_address);
    let list_call_object = Arc::new(Mutex::new(HashMap::<u64, Socket>::new()));
    let id_count = Arc::new(Mutex::new(0_u64));
    let res = run_actor(ListObjects::new(), 1);

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let socket = Socket::new(socket, addr);
        let list_object_requestor = res.requestor.clone();
        let inner_list_call_object = list_call_object.clone();
        let inner_id_count = id_count.clone();
        tokio::spawn(async move {
            log::trace!("Connected: {}", socket.ip_address());

            loop {
                let mut data = Vec::new();

                match socket.read(&mut data).await {
                    Ok(usize) => {
                        if usize == 0 {
                            break;
                        }
                    }
                    Err(err) => {
                        log::error!("{:?}", err);
                        break;
                    }
                }
                let sep = util::separate(data.as_slice());
                for data in sep {
                    match serde_json::from_slice::<SocketMessage>(data.as_slice()) {
                        Ok(mut msg) => match msg.kind() {
                            MessageType::AddShareObjectRequest => {
                                let mut id = inner_id_count.lock().await;
                                *id += 1;
                                msg = msg.set_id(*id);
                                log::info!("[{}] {}", socket.ip_address(), msg);
                                let res: Result<Option<SocketMessage>, atticus::Error> =
                                    list_object_requestor
                                        .request(RequestListObjects::Add(msg, socket.clone()))
                                        .await;
                                let msg = message::result_to_socket_message(
                                    res,
                                    MessageType::AddShareObjectResponse,
                                );
                                if socket.write(msg.as_bytes().as_slice()).await.is_err() {
                                    break;
                                }
                            }
                            MessageType::AddShareObjectResponse => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                            }
                            MessageType::RemoteCallRequest => {
                                let mut lst = inner_list_call_object.lock().await;
                                let mut id = inner_id_count.lock().await;
                                *id += 1;
                                msg = msg.set_id(*id);
                                lst.insert(*id, socket.clone());

                                log::info!("[{}] {}", socket.ip_address(), msg);
                                let res = list_object_requestor
                                    .request(RequestListObjects::CallMethod(msg))
                                    .await;
                                match res {
                                    Ok(res) => {
                                        if let Some(res) = res {
                                            if res.body() != SUCCESS.as_bytes() {
                                                break;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("{:?}", err);
                                        break;
                                    }
                                }
                            }
                            MessageType::RemoteCallResponse => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                                let mut lst = inner_list_call_object.lock().await;
                                if let Some(remote) = lst.get(&msg.id()) {
                                    match serde_json::to_vec(&msg) {
                                        Ok(data) => match remote.write(&data).await {
                                            Ok(_res) => {
                                                lst.remove(&msg.id());
                                            }
                                            Err(err) => {
                                                log::error!("{:?}", err);
                                                break;
                                            }
                                        },
                                        Err(_) => todo!(),
                                    }
                                }
                            }
                            MessageType::SendEventRequest => {
                                let mut id = inner_id_count.lock().await;
                                *id += 1;
                                msg = msg.set_id(*id);
                                log::info!("[{}] {}", socket.ip_address(), msg);

                                let _ = list_object_requestor
                                    .request(RequestListObjects::SendEvent(msg))
                                    .await;
                            }
                            MessageType::SendEventResponse => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                            }
                            MessageType::SubscribeEventRequest => {
                                let mut id = inner_id_count.lock().await;
                                *id += 1;
                                msg = msg.set_id(*id);
                                log::info!("[{}] {}", socket.ip_address(), msg);

                                let _ = list_object_requestor
                                    .request(RequestListObjects::SubscribeEvent(
                                        msg,
                                        socket.clone(),
                                    ))
                                    .await;
                            }
                            MessageType::SubscribeEventResponse => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                            }
                            MessageType::RemoveShareObjectRequest => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                            }
                            MessageType::RemoveShareObjectResponse => {
                                log::info!("[{}] {}", socket.ip_address(), msg);
                            }
                            MessageType::WaitForObject => {
                                let mut id = inner_id_count.lock().await;
                                *id += 1;
                                msg = msg.set_id(*id);
                                log::info!("[{}] {}", socket.ip_address(), msg);
                                let res: Result<Option<SocketMessage>, atticus::Error> =
                                    list_object_requestor
                                        .request(RequestListObjects::WaitForObject(msg))
                                        .await;
                                let msg = message::result_to_socket_message(
                                    res,
                                    MessageType::WaitForObject,
                                );
                                if socket.write(msg.as_bytes().as_slice()).await.is_err() {
                                    break;
                                }
                            }
                        },
                        Err(error) => {
                            log::error!(
                                "Invalid message from {}: {}\nStream: {:?}",
                                socket.ip_address(),
                                error.to_string(),
                                String::from_utf8(data)
                            );
                            break;
                        }
                    }
                }
            }
            log::trace!("Disconnected: {}", socket.ip_address());
            let _ = list_object_requestor
                .request(RequestListObjects::Remove(socket.clone()))
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use crate::{
        connector::Connector,
        error::RemoteError,
        logger::setup_logger,
        shared_object::{SharedObject, SharedObjectDispatcher},
        socket::ENV_SERVER_ADDRESS,
        wait_for_object::wait_for_objects,
    };
    use async_trait::async_trait;
    use json_elem::JsonElem;
    use tokio::{runtime::Builder, sync::Mutex, task::LocalSet};

    use super::start_server;

    fn find_available_port(start_port: u16) -> Option<u16> {
        (start_port..=u16::MAX)
            .find(|&port| std::net::TcpListener::bind(("127.0.0.1", port)).is_ok())
    }

    #[ctor::ctor]
    fn setup_server() {
        let address = format!("127.0.0.1:{}", find_available_port(3000).unwrap());

        std::env::set_var(ENV_SERVER_ADDRESS, address);
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        std::thread::spawn(move || {
            let local = LocalSet::new();
            local.spawn_local(async move {
                // The server
                let server = tokio::spawn(async move {
                    setup_logger();
                    start_server().await;
                });

                let _ = server.await;
            });
            runtime.block_on(local);
        });
    }

    struct Mango;

    #[async_trait]
    impl SharedObject for Mango {
        async fn remote_call(
            &self,
            method: &str,
            param: JsonElem,
        ) -> Result<JsonElem, RemoteError> {
            log::trace!("[Mango] Method: {} Param: {:?}", method, param);

            Ok(JsonElem::String("This is my response from mango".into()))
        }
    }

    struct Orange;

    #[async_trait]
    impl SharedObject for Orange {
        async fn remote_call(
            &self,
            method: &str,
            param: JsonElem,
        ) -> Result<JsonElem, RemoteError> {
            log::trace!("[Orange] Method: {} Param: {:?}", method, param);

            Ok(JsonElem::String("This is my response from Orange".into()))
        }
    }

    #[tokio::test]
    async fn test_server_shared_object_call_method() {
        // The process that shares objects
        let process1 = tokio::spawn(async move {
            let mut shared = SharedObjectDispatcher::new().await.unwrap();

            shared
                .register_object("mango", Box::new(Mango))
                .await
                .unwrap();
            shared
                .register_object("orange", Box::new(Orange))
                .await
                .unwrap();

            let _r = shared.spawn().await;
        });

        let process2_result = Arc::new(Mutex::new(JsonElem::String(String::new())));
        let process2_result2 = process2_result.clone();
        let process2 = tokio::spawn(async move {
            wait_for_objects(vec!["mango".to_string()]).await.unwrap();
            let proxy = Connector::connect().await.unwrap();

            let mut param = HashMap::new();
            param.insert(
                "provider".to_string(),
                JsonElem::String("microsoft".to_string()),
            );

            let result = proxy
                .remote_call("mango", "login", JsonElem::HashMap(param))
                .await
                .unwrap();
            log::trace!("[Process 2]: {}", result);
            let mut actual = process2_result2.lock().await;
            *actual = result;
        });

        let process3_result = Arc::new(Mutex::new(JsonElem::String(String::new())));
        let process3_result3 = process3_result.clone();
        let process3 = tokio::spawn(async move {
            wait_for_objects(vec!["mango".to_string()]).await.unwrap();
            let proxy = Connector::connect().await.unwrap();

            let result = proxy
                .remote_call("mango", "login", JsonElem::Null)
                .await
                .unwrap();
            log::trace!("[Process 3]: {}", result);
            let mut actual = process3_result3.lock().await;
            *actual = result;
        });

        let process4_result = Arc::new(Mutex::new(JsonElem::String(String::new())));
        let process4_result4 = process4_result.clone();
        let process4 = tokio::spawn(async move {
            wait_for_objects(vec!["mango".to_string()]).await.unwrap();
            let proxy = Connector::connect().await.unwrap();

            let result = proxy
                .remote_call("mango", "login", JsonElem::Null)
                .await
                .unwrap();
            log::trace!("[Process 4]: {}", result);
            let mut actual = process4_result4.lock().await;
            *actual = result;
        });

        let process5_result = Arc::new(Mutex::new(JsonElem::String(String::new())));
        let process5_result5 = process5_result.clone();
        let process5 = tokio::spawn(async move {
            wait_for_objects(vec!["orange".to_string()]).await.unwrap();
            let proxy = Connector::connect().await.unwrap();

            let result = proxy
                .remote_call("orange", "login", JsonElem::Null)
                .await
                .unwrap();
            log::trace!("[Process 5]: {}", result);
            let mut actual = process5_result5.lock().await;
            *actual = result;
        });

        let process6_result = Arc::new(Mutex::new(JsonElem::String(String::new())));
        let process6_result6 = process6_result.clone();
        let process6 = tokio::spawn(async move {
            wait_for_objects(vec!["orange".to_string()]).await.unwrap();
            let proxy = Connector::connect().await.unwrap();

            let mut param = HashMap::new();
            param.insert(
                "provider".to_string(),
                JsonElem::String("microsoft".to_string()),
            );

            let result = proxy
                .remote_call("orange", "login", JsonElem::HashMap(param))
                .await
                .unwrap();
            log::trace!("[Process 6]: {}", result);
            let mut actual = process6_result6.lock().await;
            *actual = result;
        });

        let _ = tokio::join!(process2, process3, process4, process5, process6);
        process1.abort();

        let res2 = process2_result.lock().await;
        assert_eq!(
            *res2,
            JsonElem::String("This is my response from mango".into())
        );

        let res3 = process3_result.lock().await;
        assert_eq!(
            *res3,
            JsonElem::String("This is my response from mango".into())
        );

        let res4 = process4_result.lock().await;
        assert_eq!(
            *res4,
            JsonElem::String("This is my response from mango".into())
        );

        let res5 = process5_result.lock().await;
        assert_eq!(
            *res5,
            JsonElem::String("This is my response from Orange".into())
        );

        let res6 = process6_result.lock().await;
        assert_eq!(
            *res6,
            JsonElem::String("This is my response from Orange".into())
        );
    }
}
