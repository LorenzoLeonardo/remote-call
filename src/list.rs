use std::{collections::HashMap, sync::Arc};

use derive_deref_rs::Deref;

use tokio::sync::{Mutex, MutexGuard};

use crate::socket::Socket;

#[derive(Clone, Debug, Deref)]
pub struct ClientList(Arc<Mutex<HashMap<String, Socket>>>);

impl ClientList {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashMap::new())))
    }

    pub async fn add(&mut self, key: String, socket: Socket) {
        let mut list = self.0.lock().await;
        list.insert(key, socket);
    }

    pub async fn remove(&mut self, key: &str) {
        let mut list = self.0.lock().await;
        list.remove(key);
    }

    pub async fn iter(&self) -> MutexGuard<'_, HashMap<String, Socket>> {
        let list: MutexGuard<'_, HashMap<String, Socket>> = self.0.lock().await;
        list
    }
}
