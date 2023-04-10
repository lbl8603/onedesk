use std::{error::Error, fmt};

mod codec;
mod bytes_codec;
mod config;
pub mod id_conn;
#[path = "./protos/message.rs"]
pub mod message;
pub mod quic;
pub mod relay;
pub mod remote_channel;
#[path = "./protos/rendezvous.rs"]
pub mod rendezvous_proto;
pub mod tcp;
pub mod utils;
pub type ResultType<F, E = anyhow::Error> = anyhow::Result<F, E>;
pub use protobuf;
pub use rand;
pub use tokio;
pub use sha2;

#[derive(Debug)]
pub enum RemoteError {
    MessageError(String),
    Relay(String),
    Peer(String),
    InvalidData(String),
    CipherInit(String),
    Decrypt(String),
    Encrypt(String),
    Login(String),
    Channel(String),
    IO(std::io::Error),
    IdRepeat,          //id重复
    ServerKeyNotMatch, //服务器key不匹配
    Disconnection,     //断开
}

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // not sure that cause should be included in message
            RemoteError::MessageError(e) => write!(f, "MessageError {}", e),
            RemoteError::Relay(e) => write!(f, "Relay {}", e),
            RemoteError::Peer(e) => write!(f, "Peer {}", e),
            RemoteError::Decrypt(e) => write!(f, "Decrypt {}", e),
            RemoteError::Login(e) => write!(f, "Login {}", e),
            RemoteError::Channel(e) => write!(f, "Channel {}", e),
            RemoteError::CipherInit(e) => write!(f, "CipherInit {}", e),
            RemoteError::InvalidData(e) => write!(f, "InvalidData {}", e),
            RemoteError::IO(e) => write!(f, "IO {:?}", e),
            RemoteError::Encrypt(e) => write!(f, "Encrypt {:?}", e),
            RemoteError::IdRepeat => write!(f, "IdRepeat "),
            RemoteError::ServerKeyNotMatch => write!(f, "ServerKeyNotMatch "),
            RemoteError::Disconnection => write!(f, "Disconnection "),
        }
    }
}
impl Error for RemoteError {}
impl From<std::io::Error> for RemoteError {
    fn from(e: std::io::Error) -> Self {
        RemoteError::IO(e)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        println!("end");
    }
}
