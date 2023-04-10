//#![cfg(feature = "rustls")]
//! Commonly used code in most examples.

use quinn::{
    Certificate, CertificateChain, ClientConfig, ClientConfigBuilder, Endpoint, Incoming,
    PrivateKey, ServerConfig, ServerConfigBuilder, TransportConfig,
};
use std::{net::SocketAddr, sync::Arc};

use crate::ResultType;

/// Constructs a QUIC endpoint configured for use a client only.
///
/// ## Args
///
/// - server_certs: list of trusted certificates.
#[allow(unused)]
fn make_client_endpoint(bind_addr: SocketAddr, server_certs: &[&[u8]]) -> ResultType<Endpoint> {
    let client_cfg = configure_client(server_certs)?;
    let mut endpoint_builder = Endpoint::builder();
    endpoint_builder.default_client_config(client_cfg);
    let (endpoint, _incoming) = endpoint_builder.bind(&bind_addr)?;
    Ok(endpoint)
}

/// Constructs a QUIC endpoint configured to listen for incoming connections on a certain address
/// and port.
///
/// ## Returns
///
/// - a stream of incoming QUIC connections
/// - server certificate serialized into DER format
#[allow(unused)]
fn make_server_endpoint(
    udp_socket: tokio::net::UdpSocket,
    cert_der: &[u8],
    priv_key: &[u8],
) -> ResultType<Incoming> {
    let server_config = configure_server(cert_der, priv_key)?;
    let mut endpoint_builder = Endpoint::builder();
    endpoint_builder.listen(server_config);
    let (_endpoint, incoming) = endpoint_builder.with_socket(udp_socket.into_std()?)?;
    Ok(incoming)
}

/// Builds default quinn client config and trusts given certificates.
///
/// ## Args
///
/// - server_certs: a list of trusted certificates in DER format.
fn configure_client(server_certs: &[&[u8]]) -> ResultType<ClientConfig> {
    let mut cfg_builder = ClientConfigBuilder::default();
    for cert in server_certs {
        cfg_builder.add_certificate_authority(Certificate::from_der(cert)?)?;
    }
    Ok(cfg_builder.build())
}

/// Returns default server configuration along with its certificate.
#[allow(clippy::field_reassign_with_default)] // https://github.com/rust-lang/rust-clippy/issues/6527
fn configure_server(cert_der: &[u8], priv_key: &[u8]) -> ResultType<ServerConfig> {
    //改成传递字节
    // let cert_der = cert.serialize_der().unwrap();
    // let priv_key = cert.serialize_private_key_der();
    let priv_key = PrivateKey::from_der(priv_key)?;

    let mut transport_config = TransportConfig::default();
    transport_config.max_concurrent_uni_streams(0).unwrap();
    let mut server_config = ServerConfig::default();
    server_config.transport = Arc::new(transport_config);
    let mut cfg_builder = ServerConfigBuilder::new(server_config);
    let cert = Certificate::from_der(cert_der)?;
    cfg_builder.certificate(CertificateChain::from_certs(vec![cert]), priv_key)?;

    Ok(cfg_builder.build())
}

#[allow(unused)]
pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];

use std::ops::{Deref, DerefMut};

use bytes::Bytes;
use futures::StreamExt;
use quinn::{
    crypto::rustls::TlsSession,
    generic::{NewConnection, RecvStream, SendStream},
};
use tokio::net::UdpSocket;

pub struct QuicClient {
    endpoint: Endpoint,
}

impl QuicClient {
    //填入信任的证书
    pub fn new(bind_addr: SocketAddr, server_certs: &[&[u8]]) -> ResultType<Self> {
        Ok(Self {
            endpoint: make_client_endpoint(bind_addr, server_certs)?,
        })
    }
    pub async fn conn(&mut self, addr: SocketAddr, server_name: &str) -> ResultType<QuicStream> {
        let conn = self.endpoint.connect(&addr, server_name)?.await?;
        Ok(QuicStream::new(conn))
    }
}

pub struct QuicServer {
    incoming: Incoming,
}

impl QuicServer {
    pub fn new(udp_socket: UdpSocket, cert_der: &[u8], priv_key: &[u8]) -> ResultType<Self> {
        let incoming = make_server_endpoint(udp_socket, cert_der, priv_key)?;
        Ok(Self { incoming })
    }
    //监听连接
    pub async fn accept(&mut self) -> ResultType<Option<QuicStream>> {
        if let Some(conn) = self.incoming.next().await {
            return Ok(Some(QuicStream::new(conn.await?)));
        }
        Ok(None)
    }
}

pub struct QuicStream {
    conn: NewConnection<TlsSession>,
}
impl QuicStream {
    fn new(conn: NewConnection<TlsSession>) -> Self {
        Self { conn }
    }
    //quic一个连接有多个通道
    pub async fn streams(
        &mut self,
    ) -> Option<Result<(SendStream<TlsSession>, RecvStream<TlsSession>), quinn::ConnectionError>>
    {
        self.conn.bi_streams.next().await
    }
    //发送不可靠、无序的数据报，某些场景很有用
    pub fn send_datagram(&mut self, data: Bytes) -> Result<(), quinn::SendDatagramError> {
        self.conn.connection.send_datagram(data)
    }
    //接收对方发送的不可靠、无序的数据报
    pub async fn next_datagram(&mut self) -> Option<Result<Bytes, quinn::ConnectionError>> {
        self.conn.datagrams.next().await
    }
}
impl Deref for QuicStream {
    type Target = NewConnection<TlsSession>;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}
impl DerefMut for QuicStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}
