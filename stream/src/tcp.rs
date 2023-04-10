use std::net::{SocketAddr};

use native_tls::Certificate;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_native_tls::{TlsAcceptor, TlsConnector, TlsStream};


use crate::ResultType;

pub struct TcpTlsClient(TlsConnector);

impl TcpTlsClient {
    pub fn new_cert(cert: &[u8]) -> ResultType<TcpTlsClient> {
        let mut builder = native_tls::TlsConnector::builder();
        builder
            .disable_built_in_roots(true)
            .danger_accept_invalid_hostnames(true)
            .add_root_certificate(Certificate::from_der(cert)?);
        Ok(TcpTlsClient(TlsConnector::from(builder.build()?)))
    }
    pub fn new() -> ResultType<TcpTlsClient> {
        let mut builder = native_tls::TlsConnector::builder();
        builder
            .disable_built_in_roots(true)
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true);
        Ok(TcpTlsClient(TlsConnector::from(builder.build()?)))
        //先不校验id服务器证书
        // Ok(TcpTlsClient(TlsConnector::from(
        //     native_tls::TlsConnector::new()?,
        // )))
    }
    pub async fn connect<A: ToSocketAddrs>(&self, addr: A) -> ResultType<(TlsStream<TcpStream>, SocketAddr)> {
        let stream = TcpStream::connect(addr).await?;
        let local_addr = stream.local_addr()?;
        Ok((self.0.connect("", stream).await?, local_addr))
    }
    pub async fn connect_secure<A: ToSocketAddrs>(&self, addr: A,domain:&str) -> ResultType<(TlsStream<TcpStream>, SocketAddr)> {
        let stream = TcpStream::connect(addr).await?;
        let local_addr = stream.local_addr()?;
        Ok((self.0.connect(domain, stream).await?, local_addr))
    }
}
pub struct TcpTlsServer(TlsAcceptor);

impl TcpTlsServer {
    pub fn new(pkcs12: &[u8]) -> ResultType<TcpTlsServer> {
        let pkcs12 = native_tls::Identity::from_pkcs12(pkcs12, "")?;
        Ok(TcpTlsServer(TlsAcceptor::from(native_tls::TlsAcceptor::new(
            pkcs12,
        )?)))
    }

    pub async fn accept(&self, stream: TcpStream) -> ResultType<TlsStream<TcpStream>> {
        Ok(self.0.accept(stream).await?)
    }
}



