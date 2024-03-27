use std::{net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

pub const CHUNK_SIZE: usize = 4096;
pub const ENV_SERVER_ADDRESS: &str = "ENV_SERVER_ADDRESS";
pub const SERVER_ADDRESS: &str = "127.0.0.1:1986";

#[derive(Clone, Debug)]
pub struct Socket {
    read: Arc<Mutex<OwnedReadHalf>>,
    write: Arc<Mutex<OwnedWriteHalf>>,
    ip_address: SocketAddr,
}

impl Socket {
    pub fn new(socket: TcpStream, ip_address: SocketAddr) -> Self {
        let (read, write) = socket.into_split();
        Self {
            read: Arc::new(Mutex::new(read)),
            write: Arc::new(Mutex::new(write)),
            ip_address,
        }
    }

    pub async fn read(&self, data: &mut Vec<u8>) -> Result<usize, std::io::Error> {
        let mut read = self.read.lock().await;

        let mut reader = BufReader::new(&mut *read);
        loop {
            let mut buffer = Vec::new();
            match reader.read_until(b'\n', &mut buffer).await {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionReset,
                            "The connection was reset by the remote server.",
                        ));
                    }
                    data.extend_from_slice(&buffer[0..bytes_read]);

                    if buffer.ends_with(&[b'\n']) {
                        return Ok(data.len());
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    pub async fn write(&self, data: &[u8]) -> Result<(), std::io::Error> {
        let mut write = self.write.lock().await;

        let mut stream = data.to_vec();
        stream.push(b'\n');
        write.write_all(stream.as_slice()).await
    }

    pub fn ip_address(&self) -> String {
        self.ip_address.to_string()
    }
}
