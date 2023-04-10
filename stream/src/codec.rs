use bytes::{Buf, BufMut, BytesMut};
use rand::Rng;
use std::io;
use tokio_util::codec::{Decoder, Encoder};

use crate::{utils::Aes128GcmUtil, RemoteError};

//标志 用一个和quic不一样的，用于区分两种协议 https://cloud.tencent.com/developer/article/1387659
pub const FLAG: u8 = 0x05;

#[derive(Clone)]
pub struct TcpBytesCodec {
    state: DecodeState,
    aes: Option<Aes128GcmUtil>,
    nonce1: Option<Vec<u8>>,
    nonce2: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
enum DecodeState {
    Head,
    Data(usize),
}

impl TcpBytesCodec {
    pub fn new() -> Self {
        Self {
            state: DecodeState::Head,
            aes: None,
            nonce1: None,
            nonce2: None,
        }
    }
    pub fn set_aes(&mut self, key_bytes: &[u8], nonce: Vec<u8>) -> Result<(), RemoteError> {
        self.aes = Some(Aes128GcmUtil::new(key_bytes)?);
        self.nonce1 = Some(nonce.clone());
        self.nonce2 = Some(nonce);
        Ok(())
    }

    fn decode_head(&mut self, src: &mut BytesMut) -> io::Result<Option<usize>> {
        if src.len() < 2 {
            return Ok(None);
        }
        //版本号
        let version = src[0];
        if version != FLAG {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "标志不一致"));
        }
        //低位是标志
        let head_len = ((src[1] & 0x1) + 2) as usize;
        //使用小端存储
        let mut data_len = src[1] as usize;
        if head_len > 2 {
            data_len |= (src[2] as usize) << 8;
        }
        data_len >>= 1;
        //读取完头部 要往前移
        src.advance(head_len);
        //保证缓冲区够长
        src.reserve(data_len);
        return Ok(Some(data_len));
    }

    fn decode_data(&self, n: usize, src: &mut BytesMut) -> Option<BytesMut> {
        if src.len() < n {
            return None;
        }
        Some(src.split_to(n))
    }
}

impl Decoder for TcpBytesCodec {
    type Item = Vec<u8>;
    type Error = crate::RemoteError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Vec<u8>>, Self::Error> {
        let n = match self.state {
            DecodeState::Head => match self.decode_head(src)? {
                Some(n) => {
                    self.state = DecodeState::Data(n);
                    n
                }
                None => return Ok(None),
            },
            DecodeState::Data(n) => n,
        };

        match self.decode_data(n, src) {
            Some(data) => {
                self.state = DecodeState::Head;
                if let Some(aes) = &self.aes {
                    if let Some(nonce2) = &self.nonce2{
                        let mut data = aes.decrypt(&data, &nonce2)?;
                        if let Some(last) = data.pop() {
                            // let mut iter = nonce2.iter_mut();
                            // while let Some(s) = iter.next() {
                            //     *s ^= last;
                            // }
                            return Ok(Some(data));
                        } else {
                            return Err(RemoteError::Decrypt(String::from("解密失败")));
                        }
                    } else {
                        return Err(RemoteError::Decrypt(String::from("解密失败")));
                    }
                }
                Ok(Some(data.to_vec()))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<Vec<u8>> for TcpBytesCodec {
    type Error = crate::RemoteError;

    fn encode(&mut self, mut data: Vec<u8>, buf: &mut BytesMut) -> Result<(), Self::Error> {
        if let Some(aes) = &self.aes {
            if let Some(nonce1) = &self.nonce1{
               
                let random = rand::thread_rng().gen::<u8>();
                data.push(random);
                let encrypt_data = aes.encrypt(&data, &nonce1)?;
                // log::info!("加密消息：{:?}",data);
                // log::info!("加密消息 random{:?},nonce:{:?}", random,nonce1);
                if data.len() <= 0x7F {
                    buf.reserve(encrypt_data.len() + 2);
                    buf.put_u8(FLAG);
                    buf.put_u8((encrypt_data.len() << 1) as u8);
                } else if encrypt_data.len() <= 0x7FFF {
                    buf.reserve(encrypt_data.len() + 3);
                    buf.put_u8(FLAG);
                    //用小端计数（低地址存低位），左边存标志
                    buf.put_u16_le((encrypt_data.len() << 1) as u16 | 0x1);
                } else {
                    //太长
                    return Err(Self::Error::InvalidData(String::from("Overflow")));
                }
 
                // let mut iter = nonce1.iter_mut();
                // while let Some(s) = iter.next() {
                //     *s ^= random;
                // }
                buf.extend(encrypt_data);
                return Ok(());
            } else {
                return Err(RemoteError::Encrypt(String::from("加密失败")));
            }
        }
        if data.len() <= 0x7F {
            buf.reserve(data.len() + 2);
            buf.put_u8(FLAG);
            buf.put_u8((data.len() << 1) as u8);
        } else if data.len() <= 0x7FFF {
            buf.reserve(data.len() + 3);
            buf.put_u8(FLAG);
            //用小端计数（低地址存低位），左边存标志
            buf.put_u16_le((data.len() << 1) as u16 | 0x1);
        } else {
            //太长
            return Err(Self::Error::InvalidData(String::from("Overflow")));
        }
        buf.extend(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_codec1() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.resize(0x3F, 1);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        let buf_saved = buf.clone();
        assert_eq!(buf.len(), 0x3F + 1);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3F);
            assert_eq!(res[0], 1);
        } else {
            assert!(false);
        }
        let mut codec2 = TcpBytesCodec::new();
        let mut buf2 = BytesMut::new();
        if let Ok(None) = codec2.decode(&mut buf2) {
        } else {
            assert!(false);
        }
        buf2.extend(&buf_saved[0..1]);
        if let Ok(None) = codec2.decode(&mut buf2) {
        } else {
            assert!(false);
        }
        buf2.extend(&buf_saved[1..]);
        if let Ok(Some(res)) = codec2.decode(&mut buf2) {
            assert_eq!(res.len(), 0x3F);
            assert_eq!(res[0], 1);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_codec2() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        assert!(!codec.encode("".into(), &mut buf).is_err());
        assert_eq!(buf.len(), 1);
        bytes.resize(0x3F + 1, 2);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        assert_eq!(buf.len(), 0x3F + 2 + 2);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0);
        } else {
            assert!(false);
        }
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3F + 1);
            assert_eq!(res[0], 2);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_codec3() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.resize(0x3F - 1, 3);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        assert_eq!(buf.len(), 0x3F + 1 - 1);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3F - 1);
            assert_eq!(res[0], 3);
        } else {
            assert!(false);
        }
    }
    #[test]
    fn test_codec4() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.resize(0x3FFF, 4);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        assert_eq!(buf.len(), 0x3FFF + 2);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3FFF);
            assert_eq!(res[0], 4);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_codec5() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.resize(0x3FFFFF, 5);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        assert_eq!(buf.len(), 0x3FFFFF + 3);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3FFFFF);
            assert_eq!(res[0], 5);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn test_codec6() {
        let mut codec = TcpBytesCodec::new();
        let mut buf = BytesMut::new();
        let mut bytes: Vec<u8> = Vec::new();
        bytes.resize(0x3FFFFF + 1, 6);
        assert!(!codec.encode(bytes.into(), &mut buf).is_err());
        let buf_saved = buf.clone();
        assert_eq!(buf.len(), 0x3FFFFF + 4 + 1);
        if let Ok(Some(res)) = codec.decode(&mut buf) {
            assert_eq!(res.len(), 0x3FFFFF + 1);
            assert_eq!(res[0], 6);
        } else {
            assert!(false);
        }
        let mut codec2 = TcpBytesCodec::new();
        let mut buf2 = BytesMut::new();
        buf2.extend(&buf_saved[0..1]);
        if let Ok(None) = codec2.decode(&mut buf2) {
        } else {
            assert!(false);
        }
        buf2.extend(&buf_saved[1..6]);
        if let Ok(None) = codec2.decode(&mut buf2) {
        } else {
            assert!(false);
        }
        buf2.extend(&buf_saved[6..]);
        if let Ok(Some(res)) = codec2.decode(&mut buf2) {
            assert_eq!(res.len(), 0x3FFFFF + 1);
            assert_eq!(res[0], 6);
        } else {
            assert!(false);
        }
    }
}
