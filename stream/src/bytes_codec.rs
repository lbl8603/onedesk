use std::{io::{self, Read, Write}, net::{Shutdown, TcpStream}};

use bytes::BufMut;
use rand::Rng;

use crate::{utils::Aes128GcmUtil, RemoteError};
pub const FLAG: u8 = 0x05;

pub fn decode(tcp: &mut impl Read) -> Result<Vec<u8>, RemoteError> {
    //加上版本号头部至少有两个字节
    let buf = read(tcp, 2)?;
    //版本号
    if buf[0] != FLAG {
        Err(io::Error::new(io::ErrorKind::InvalidData, "标志不一致"))?
    }
    //低两位是头部长度-1，因为前面读了一个字节所以这里不再额外处理
    let head_len = (buf[1] & 0x3) as usize;
    let mut data_len = buf[1] as usize;
    if head_len > 0 {
        let head_data = read(tcp, head_len)?;
        data_len |= (head_data[0] as usize) << 8;
        if head_len > 1 {
            data_len |= (head_data[1] as usize) << 16;
        }
        if head_len > 2 {
            data_len |= (head_data[2] as usize) << 24;
        }
    };
    data_len >>= 2;
    let body_data = read(tcp, data_len)?;
    Ok(body_data)
}
fn read(tcp: &mut impl Read, size: usize) -> Result<Vec<u8>, RemoteError> {
    let mut buf = Vec::with_capacity(size);
    let mut index = 0;
    loop {
        let current_buf = &mut buf[index..1024];
        let len = tcp.read(current_buf)?;
        if len == 0 {
            return Err(RemoteError::Disconnection);
        }
        index += len;
        if index == size {
            return Ok(buf[..size].to_vec());
        }
    }
}

pub fn encode(data: Vec<u8>) -> Result<Vec<u8>, RemoteError> {
    //插入版本号
    let mut out = Vec::new();
    out.put_u8(FLAG);
    // out.insert(index, element)
    if data.len() <= 0x3F {
        out.put_u8((data.len() << 2) as u8);
    } else if data.len() <= 0x3FFF {
        out.put_u16_le((data.len() << 2) as u16 | 0x1);
    } else if data.len() <= 0x3FFFFF {
        let h = (data.len() << 2) as u32 | 0x2;
        out.put_u16_le((h & 0xFFFF) as u16);
        out.put_u8((h >> 16) as u8);
    } else if data.len() <= 0x3FFFFFFF {
        out.put_u32_le((data.len() << 2) as u32 | 0x3);
    } else {
        return Err(RemoteError::InvalidData(String::from("Overflow")));
    }
    out.extend(data);
    Ok(out)
}

pub struct TcpFramed {
    tcp: TcpStream,
    aes: Option<Aes128GcmUtil>,
    nonce1: Option<Vec<u8>>,
    nonce2: Option<Vec<u8>>,
}
impl TcpFramed {
    pub fn new(tcp: TcpStream) -> Self {
        Self {
            tcp,
            aes: None,
            nonce1: None,
            nonce2: None,
        }
    }
    pub fn close(self)-> Result<(), RemoteError>{
        self.tcp.shutdown(Shutdown::Both)?;
        Ok(())
    }
    pub fn try_clone(&self) -> Result<Self, RemoteError> {
        let tcp = self.tcp.try_clone()?;
        Ok(Self {
            tcp,
            aes: self.aes.clone(),
            nonce1: self.nonce1.clone(),
            nonce2: self.nonce2.clone(),
        })
    }
    pub fn set_aes(&mut self, key_bytes: &[u8], nonce: Vec<u8>) -> Result<(), RemoteError> {
        self.aes = Some(Aes128GcmUtil::new(key_bytes)?);
        self.nonce1 = Some(nonce.clone());
        self.nonce2 = Some(nonce);
        Ok(())
    }
    pub fn next(&mut self) -> Result<Vec<u8>, RemoteError> {
        let data = decode(&mut self.tcp)?;
        if let Some(aes) = &self.aes {
            if let Some(nonce2) = &self.nonce2 {
                let mut data = aes.decrypt(&data, &nonce2)?;
                if let Some(_last) = data.pop() {
                    // let mut iter = nonce2.iter_mut();
                    // while let Some(s) = iter.next() {
                    //     *s ^= last;
                    // }
                    return Ok(data);
                } else {
                    return Err(RemoteError::Decrypt(String::from("解密失败")));
                }
            } else {
                return Err(RemoteError::Decrypt(String::from("解密失败")));
            }
        }
        return Ok(data);
    }
    pub fn send(&mut self, mut data: Vec<u8>) -> Result<usize, RemoteError> {
        if let Some(aes) = &self.aes {
            if let Some(nonce1) = &self.nonce1 {
                let random = rand::thread_rng().gen::<u8>();
                data.push(random);
                let encrypt_data = aes.encrypt(&data, &nonce1)?;
                return Ok(self.tcp.write(&encode(encrypt_data)?)?);
            }
        }
        Ok(self.tcp.write(&encode(data)?)?)
    }
}
