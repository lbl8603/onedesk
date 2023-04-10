use std::net::TcpStream;

use protobuf::Message;
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::{
    bytes_codec::TcpFramed,
    message::{
        login_response, remote_message, ClientHello, LoginRequest, LoginResponse, RemoteMessage,
        ServerHello,
    },
    rendezvous_proto::{relay_message, RelayConn, RelayMessage},
    utils::{Aes128GcmUtil, RsaPrivKey, RsaPubKey},
    RemoteError, ResultType,
};

fn relay_start(
    relay_addr: String,
    relay_id: String,
    relay_pub_key: Vec<u8>,
) -> ResultType<(TcpFramed, u64)> {
    let pub_key = RsaPubKey::new(relay_pub_key)?;
    let mut conn = RelayMessage::new();
    let key = rand::thread_rng().gen::<[u8; 16]>();
    let nonce = rand::thread_rng().gen::<[u8; 12]>();
    conn.set_relay_conn(RelayConn {
        relay_id,
        key: key.to_vec(),
        nonce: nonce.to_vec(),
        ..Default::default()
    });
    let aes = Aes128GcmUtil::new(&key)?;
    //加密后传输
    let conn_bytes = pub_key.pub_key_encrypt(&conn.write_to_bytes()?)?;
    log::info!("连接：{:?}", relay_addr);
    let mut tcp_framed = TcpFramed::new(TcpStream::connect(relay_addr)?);
    // let mut framed = Framed::new(r, TcpBytesCodec::new());
    tcp_framed.send(conn_bytes)?;
    drop(pub_key);
    drop(key);
    drop(conn);
    //取中继服务器随机数
    let relay_rand = if let Ok(rs) = tcp_framed.next() {
        if let Ok(rs) = aes.decrypt(&rs, &nonce) {
            drop(nonce);
            drop(aes);
            let msg = RelayMessage::parse_from_bytes(&rs)?;
            if let Some(relay_message::Union::relay_start(relay_start)) = msg.union {
                relay_start.rand
            } else {
                Err(RemoteError::Relay(String::from("中继服务器消息错误")))?
            }
        } else {
            Err(RemoteError::Relay(String::from("中继服务器消息解密失败")))?
        }
    } else {
        Err(RemoteError::Relay(String::from("断开连接")))?
    };
    Ok((tcp_framed, relay_rand))
}
pub struct RelayClient;

#[derive(Debug, Clone, Copy)]
pub enum LoginResponseEnum {
    First,
    NotMatch,
    Frequently,
}
impl RelayClient {
    pub fn start<F>(
        my_id: String,
        peer_password_fn: F,
        relay_addr: String,
        relay_id: String,
        peer_id: String,
        peer_pub_key: Vec<u8>,
        relay_pub_key: Vec<u8>,
    ) -> ResultType<TcpFramed>
    where
        F: Fn(String, String, LoginResponseEnum) -> Option<Vec<u8>> + 'static,
    {
        let (tcp_framed, mut relay_rand) = relay_start(relay_addr, relay_id, relay_pub_key)?;
        if relay_rand == 0 {
            relay_rand = rand::thread_rng().gen()
        }
        RelayClient::connect_peer(
            my_id,
            peer_password_fn,
            peer_id,
            peer_pub_key,
            relay_rand,
            tcp_framed,
        )
    }
    fn connect_peer<F>(
        my_id: String,
        peer_password_fn: F,
        peer_id: String,
        peer_pub_key: Vec<u8>,
        relay_rand: u64,
        mut tcp_framed: TcpFramed,
    ) -> ResultType<TcpFramed>
    where
        F: Fn(String, String, LoginResponseEnum) -> Option<Vec<u8>> + 'static,
    {
        //剩下128位由被控方返回
        let my_rand: u64 = rand::thread_rng().gen();
        let rand1 = (my_rand << 32) | (relay_rand >> 32);
        let rand2 = (relay_rand << 32) | (my_rand >> 32);
        log::info!("生成随机数 rand1:{},rand2:{}", rand1, rand2);
        let mut client_hello = RemoteMessage::new();
        let key = rand::thread_rng().gen::<[u8; 16]>();
        let nonce = rand::thread_rng().gen::<[u8; 12]>();
        client_hello.set_client_hello(ClientHello {
            my_id,
            peer_id: peer_id.clone(),
            rand1,
            rand2,
            key: key.to_vec(),
            nonce: nonce.to_vec(),
            ..Default::default()
        });
        let aes = Aes128GcmUtil::new(&key)?;
        let peer_pub_key = RsaPubKey::new(peer_pub_key)?;
        let bytes = peer_pub_key.pub_key_encrypt(&client_hello.write_to_bytes()?)?;
        log::info!("client_hello::{:?}", client_hello);
        tcp_framed.send(bytes)?;
        let (peer_rand1, peer_rand2, hash) = if let Ok(server_hello) = tcp_framed.next() {
            if let Ok(server_hello) = aes.decrypt(&server_hello, &nonce) {
                let msg = RemoteMessage::parse_from_bytes(&server_hello)?;
                log::info!("server_hello::{:?}", msg);
                if let Some(remote_message::Union::server_hello(server_hello)) = msg.union {
                    (server_hello.rand1, server_hello.rand2, server_hello.hash)
                } else {
                    Err(RemoteError::Peer(String::from("被控方消息错误")))?
                }
            } else {
                Err(RemoteError::Decrypt(String::from("被控方消息解密失败")))?
            }
        } else {
            Err(RemoteError::Peer(String::from("断开连接")))?
        };
        let key = ((peer_rand1 as u128) << 64) | (rand1 as u128 >> 64);
        let nonce = ((peer_rand2 as u128) << 64) | (rand2 as u128 >> 64);
        log::info!("协商密钥 key:{},nonce:{}", key, nonce);
        tcp_framed.set_aes(&key.to_le_bytes(), nonce.to_le_bytes()[..12].to_vec())?;
        let mut check_info = String::new();
        let mut login_enum = LoginResponseEnum::First;
        loop {
            let peer_password = if let Some(peer_password) =
                peer_password_fn(peer_id.clone(), check_info.clone(), login_enum)
            {
                peer_password
            } else {
                Err(RemoteError::Login(String::from("关闭")))?
            };
            let mut hasher2 = Sha256::new();
            hasher2.update(&hash);
            hasher2.update(peer_password);
            let my_hash = rand::thread_rng().gen::<[u8; 16]>();
            hasher2.update(my_hash);
            let mut login = RemoteMessage::new();
            login.set_login_request(LoginRequest {
                password: hasher2.finalize()[..].into(),
                hash: my_hash.into(),
                ..Default::default()
            });
            log::info!("登录消息:{:?}", login);
            tcp_framed.send(login.write_to_bytes()?)?;
            if let Ok(login_response) = tcp_framed.next() {
                let msg = RemoteMessage::parse_from_bytes(&login_response)?;
                if let Some(remote_message::Union::login_response(login_response)) = msg.union {
                    log::info!("登录消息回应:{:?}", login_response);
                    match login_response.code.enum_value_or_default() {
                        crate::message::login_response::Code::Success => {
                            break;
                        }
                        crate::message::login_response::Code::NotMatch => {
                            check_info = login_response.error;
                            login_enum = LoginResponseEnum::NotMatch;
                        }
                        crate::message::login_response::Code::Frequently => {
                            check_info = login_response.error;
                            login_enum = LoginResponseEnum::Frequently;
                        }
                    }
                } else {
                    Err(RemoteError::Login(String::from("被控方消息错误")))?
                }
            } else {
                Err(RemoteError::Peer(String::from("断开连接")))?
            }
        }

        //登录成功
        Ok(tcp_framed)
    }
}

pub struct RelayServer;
impl RelayServer {
    pub fn start(
        my_id: String,
        my_password: &[u8],
        my_priv_key: &RsaPrivKey,
        relay_addr: String,
        relay_id: String,
        peer_id: String,
        relay_pub_key: Vec<u8>,
    ) -> ResultType<TcpFramed> {
        let (framed, _) = relay_start(relay_addr, relay_id, relay_pub_key)?;
        RelayServer::connect_peer(my_id, my_password, my_priv_key, peer_id, framed)
    }
    fn connect_peer(
        my_id: String,
        my_password: &[u8],
        my_priv_key: &RsaPrivKey,
        peer_id: String,
        mut framed: TcpFramed,
    ) -> ResultType<TcpFramed> {
        if let Ok(client_hello) = framed.next() {
            if let Ok(client_hello) = my_priv_key.priv_key_decrypt(&client_hello) {
                let msg = RemoteMessage::parse_from_bytes(&client_hello)?;
                if let Some(remote_message::Union::client_hello(client_hello)) = msg.union {
                    // let my_id = client_hello.my_id;
                    if client_hello.my_id != peer_id || client_hello.peer_id != my_id {
                        Err(RemoteError::Peer(String::from("主控方消息错误")))?
                    }
                    let aes = Aes128GcmUtil::new(&client_hello.key)?;
                    let mut server_hello = RemoteMessage::new();
                    let peer_rand1 = client_hello.rand1;
                    let peer_rand2 = client_hello.rand2;
                    let rand1: u64 = rand::thread_rng().gen();
                    let rand2: u64 = rand::thread_rng().gen();
                    let hash = rand::thread_rng().gen::<[u8; 16]>();
                    server_hello.set_server_hello(ServerHello {
                        rand1,
                        rand2,
                        hash: hash.to_vec(),
                        ..Default::default()
                    });
                    framed
                        .send(aes.encrypt(&server_hello.write_to_bytes()?, &client_hello.nonce)?)?;

                    let key = ((rand1 as u128) << 64) | (peer_rand1 as u128 >> 64);
                    let nonce = ((rand2 as u128) << 64) | (peer_rand2 as u128 >> 64);
                    log::info!("协商密钥 key:{},nonce:{}", key, nonce);
                    framed.set_aes(&key.to_le_bytes(), nonce.to_le_bytes()[..12].to_vec())?;
                    //验证密码
                    let mut check = 0;
                    loop {
                        if let Ok(password) = framed.next() {
                            let msg = RemoteMessage::parse_from_bytes(&password)?;
                            log::info!("登录消息:{:?}", msg);
                            if let Some(remote_message::Union::login_request(login_request)) =
                                msg.union
                            {
                                let mut hasher2 = Sha256::new();
                                hasher2.update(hash);
                                hasher2.update(my_password);
                                hasher2.update(login_request.hash);
                                let password_hash = hasher2.finalize()[..].to_vec();
                                let mut login_response = RemoteMessage::new();
                                if password_hash != login_request.password {
                                    if check > 5 {
                                        login_response.set_login_response(LoginResponse {
                                            code: protobuf::ProtobufEnumOrUnknown::from(
                                                login_response::Code::Frequently,
                                            ),
                                            error: String::from("密码错误,尝试密码次数多"),
                                            ..Default::default()
                                        });
                                        framed.send(login_response.write_to_bytes()?)?;
                                        Err(RemoteError::Login(String::from("尝试密码次数多")))?
                                    } else {
                                        login_response.set_login_response(LoginResponse {
                                            code: protobuf::ProtobufEnumOrUnknown::from(
                                                login_response::Code::NotMatch,
                                            ),
                                            error: String::from("密码错误"),
                                            ..Default::default()
                                        });
                                        framed.send(login_response.write_to_bytes()?)?;
                                    }
                                } else {
                                    login_response.set_login_response(LoginResponse {
                                        code: protobuf::ProtobufEnumOrUnknown::from(
                                            login_response::Code::Success,
                                        ),
                                        ..Default::default()
                                    });
                                    framed.send(login_response.write_to_bytes()?)?;
                                    break;
                                }
                                check += 1;
                            } else {
                                Err(RemoteError::Login(String::from("主控方消息错误")))?
                            }
                        } else {
                            Err(RemoteError::Peer(String::from("断开连接")))?
                        }
                    }
                } else {
                    Err(RemoteError::Peer(String::from("主控方消息错误")))?
                }
            } else {
                Err(RemoteError::Decrypt(String::from("主控方消息解密失败")))?
            }
        } else {
            Err(RemoteError::Peer(String::from("断开连接")))?
        };

        Ok(framed)
    }
}
