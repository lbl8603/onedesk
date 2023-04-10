use std::time::Duration;

use futures::{SinkExt, StreamExt};
use protobuf::Message;
use tokio::{
    net::{TcpStream, ToSocketAddrs},
    sync::mpsc::Receiver,
    time,
};
use tokio_native_tls::TlsStream;
use tokio_util::codec::Framed;

use crate::{
    codec::TcpBytesCodec,
    rendezvous_proto::{
        register_peer_response, relay_response, rendezvous_message, RendezvousMessage,
    },
    tcp::TcpTlsClient,
    utils::Cert,
    RemoteError, ResultType,
};

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum CallbackCode {
    MsgError(String), //消息错误
    PeerOffline,      //对方不在线
}

pub struct IdClient {
    user_id: String,
    server_key: String,
    cert: Cert,
    pub_key: Vec<u8>,
    error_callback: fn(msg: CallbackCode),
    control_server_callback:
        fn(peer_id: String, relay_id: String, relay_addr: String, relay_pub_key: Vec<u8>),
    control_client_callback: fn(
        peer_id: String,
        peer_pub_key: Vec<u8>,
        relay_id: String,
        relay_addr: String,
        relay_pub_key: Vec<u8>,
    ),
}

impl IdClient {
    pub fn new(
        user_id: String,
        server_key: String,
        cert: Cert,
        pub_key: Vec<u8>,
        error_callback: fn(msg: CallbackCode),
        control_server_callback: fn(
            peer_id: String,
            relay_id: String,
            relay_addr: String,
            relay_pub_key: Vec<u8>,
        ),
        control_client_callback: fn(
            peer_id: String,
            peer_pub_key: Vec<u8>,
            relay_id: String,
            relay_addr: String,
            relay_pub_key: Vec<u8>,
        ),
    ) -> Self {
        Self {
            user_id,
            server_key,
            cert,
            pub_key,
            error_callback,
            control_server_callback,
            control_client_callback,
        }
    }
    pub async fn start<A: ToSocketAddrs>(
        &self,
        addr: A,
        domain: &str,
        receiver: Receiver<Vec<u8>>,
    ) -> ResultType<impl futures::Future<Output = Result<(), anyhow::Error>>> {
        let result = IdClient::start_(self, addr, domain, receiver).await;
        result
    }

    async fn start_<A: ToSocketAddrs>(
        &self,
        addr: A,
        domain: &str,
        receiver: Receiver<Vec<u8>>,
    ) -> ResultType<impl futures::Future<Output = Result<(), anyhow::Error>>> {
        let client = TcpTlsClient::new()?;
        let (tcp_stream, _local_addr) = client.connect_secure(addr, domain).await?;
        let mut framed = Framed::new(tcp_stream, TcpBytesCodec::new());
        let mut register = RendezvousMessage::new();
        register.set_register_peer(crate::rendezvous_proto::RegisterPeer {
            user_id: self.user_id.clone(),
            server_key: self.server_key.clone(),
            cert: self.cert.cert_der()?,
            pub_key: self.pub_key.clone(),
            ..Default::default()
        });
        framed.send(register.write_to_bytes()?).await?;
        if let Some(bytes_mut) = framed.next().await {
            let msg = RendezvousMessage::parse_from_bytes(&bytes_mut?)?;
            if let Some(rendezvous_message::Union::register_peer_response(msg)) = msg.union {
                match msg.code.enum_value_or_default() {
                    register_peer_response::Code::Success => {}
                    register_peer_response::Code::Fail => {
                        Err(RemoteError::MessageError(msg.message))?
                    }
                    register_peer_response::Code::Error => {
                        Err(RemoteError::MessageError(msg.message))?
                    }

                    register_peer_response::Code::Repeat => Err(RemoteError::IdRepeat)?,
                    register_peer_response::Code::KeyNotMatch => {
                        Err(RemoteError::ServerKeyNotMatch)?
                    }
                }
            } else {
                Err(RemoteError::MessageError(String::from("消息错误")))?
            }
        } else {
            Err(RemoteError::Disconnection)?
        }
        let error_callback = self.error_callback;
        let control_server_callback = self.control_server_callback;
        let control_client_callback = self.control_client_callback;
        let a = IdClient::loop_(
            error_callback,
            control_server_callback,
            control_client_callback,
            receiver,
            framed,
        );
        return Ok(a);

        // self.status = 2;
        // //维持心跳 heartbeat
        // let mut interval = time::interval(Duration::from_millis(500));
        // let mut ping = RendezvousMessage::new();
        // ping.set_ping(crate::rendezvous_proto::Ping {
        //     ..Default::default()
        // });
        // let ping_v = ping.write_to_bytes()?;
        // drop(ping);
        // loop {
        //     tokio::select! {
        //         bytes = receiver.recv() =>{
        //             if let Some(bytes) = bytes {
        //                 framed.send(bytes).await?;
        //             }else{
        //                 return Ok(StartCode::Disconnection)
        //             }
        //         },
        //         _ = interval.tick() =>{
        //             framed.send(ping_v.clone()).await?;
        //         },
        //         bytes_mut = framed.next()=>{
        //             if let Some(bytes_mut) = bytes_mut {
        //                 let msg = RendezvousMessage::parse_from_bytes(&bytes_mut?)?;
        //                 match msg.union {
        //                     Some(rendezvous_message::Union::relay_response(msg)) => {
        //                         //中继
        //                         match msg.code.enum_value_or_default(){
        //                             relay_response::Code::Success => {
        //                                 if msg.is_control{
        //                                     control_server_callback(msg.peer_id,msg.ralay_id,msg.relay_addr,msg.ralay_pub_key);
        //                                 }else{
        //                                     control_client_callback(msg.peer_id,msg.peer_pub_key,msg.ralay_id,msg.relay_addr,msg.ralay_pub_key);
        //                                 }
        //                             },
        //                             relay_response::Code::Fail => {
        //                                 error_callback(CallbackCode::MsgError(msg.message));
        //                             },
        //                             relay_response::Code::Offline => error_callback(CallbackCode::PeerOffline),
        //                         }
        //                     },
        //                     Some(s) => {
        //                         log::info!("id消息类型错误：{:?}",s);
        //                     },
        //                     None => {
        //                         log::info!("id消息为空");
        //                     },
        //                 }
        //             }else{
        //                 return Ok(StartCode::Disconnection)
        //             }
        //         }
        //     }
        // }
    }
    async fn loop_(
        error_callback: fn(CallbackCode),
        control_server_callback: fn(String, String, String, Vec<u8>),
        control_client_callback: fn(String, Vec<u8>, String, String, Vec<u8>),
        mut receiver: Receiver<Vec<u8>>,
        mut framed: Framed<TlsStream<TcpStream>, TcpBytesCodec>,
    ) -> ResultType<()> {
        //维持心跳 heartbeat
        let mut interval = time::interval(Duration::from_millis(500));
        let mut ping = RendezvousMessage::new();
        ping.set_ping(crate::rendezvous_proto::Ping {
            ..Default::default()
        });
        let ping_v = ping.write_to_bytes()?;
        drop(ping);
        loop {
            tokio::select! {
                bytes = receiver.recv() =>{
                    if let Some(bytes) = bytes {
                        framed.send(bytes).await?;
                    }else{
                        Err(RemoteError::Disconnection)?
                    }
                },
                _ = interval.tick() =>{
                    framed.send(ping_v.clone()).await?;
                },
                bytes_mut = framed.next()=>{
                    if let Some(bytes_mut) = bytes_mut {
                        let msg = RendezvousMessage::parse_from_bytes(&bytes_mut?)?;
                        log::info!("msg:{:?}",msg);
                        match msg.union {
                            Some(rendezvous_message::Union::relay_response(msg)) => {
                                //中继
                                match msg.code.enum_value_or_default(){
                                    relay_response::Code::Success => {
                                        if msg.is_control{
                                            control_server_callback(msg.peer_id,msg.ralay_id,msg.relay_addr,msg.ralay_pub_key);
                                        }else{
                                            control_client_callback(msg.peer_id,msg.peer_pub_key,msg.ralay_id,msg.relay_addr,msg.ralay_pub_key);
                                        }
                                    },
                                    relay_response::Code::Fail => {
                                        error_callback(CallbackCode::MsgError(msg.message));
                                    },
                                    relay_response::Code::Offline => error_callback(CallbackCode::PeerOffline),
                                }
                            },
                            Some(s) => {
                                log::info!("id消息类型错误：{:?}",s);
                            },
                            None => {
                                log::info!("id消息为空");
                            },
                        }
                    }else{
                        Err(RemoteError::Disconnection)?
                    }
                }
            }
        }
    }
}
