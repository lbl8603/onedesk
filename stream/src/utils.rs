use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes128Gcm, Key, Nonce};

use p12::PFX;
use rcgen::{Certificate, RcgenError};
use rsa::{
    pkcs1::{FromRsaPublicKey, ToRsaPublicKey},
    PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey,
};

#[derive(Clone)]
pub struct Aes128GcmUtil(Aes128Gcm);
impl Aes128GcmUtil {
    pub fn new(key_bytes: &[u8]) -> Result<Self, RemoteError> {
        if key_bytes.len() != 16 {
            Err(RemoteError::CipherInit(String::from("key位数错误")))
        } else {
            let key = Key::from_slice(key_bytes);
            let cipher = Aes128Gcm::new(key);
            Ok(Self(cipher))
        }
    }
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, RemoteError> {
        if nonce.len() != 12 {
            return Err(RemoteError::CipherInit(String::from("nonce位数错误")));
        }
        let nonce = Nonce::from_slice(nonce);
        match self.0.encrypt(nonce, plaintext.as_ref()) {
            Ok(rs) => Ok(rs),
            Err(e) => {
                log::info!("加密失败:{:?}", e);
                Err(RemoteError::Encrypt(String::from("加密失败")))
            }
        }
    }
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, RemoteError> {
        let nonce = Nonce::from_slice(nonce);
        match self.0.decrypt(nonce, ciphertext.as_ref()) {
            Ok(rs) => Ok(rs),
            Err(e) => {
                log::info!("解密失败:{:?}", e);
                Err(RemoteError::Decrypt(String::from("解密失败")))
            }
        }
    }
}

use crate::{RemoteError, ResultType};

pub struct Cert(Certificate);
impl Cert {
    pub fn new() -> ResultType<Self> {
        Ok(Cert(rcgen::generate_simple_self_signed(vec![
            "localhost".into()
        ])?))
    }
    pub fn pkcs12(&self) -> ResultType<Vec<u8>> {
        let cert_der = self.0.serialize_der()?;
        let key_der = self.0.serialize_private_key_der();
        let pfx = PFX::new(&cert_der, &key_der, None, "", "localhost");
        Ok(pfx
            .ok_or_else(|| RcgenError::KeyGenerationUnavailable)?
            .to_der())
    }
    pub fn cert_der(&self) -> ResultType<Vec<u8>> {
        Ok(self.0.serialize_der()?)
    }
}

#[derive(Debug, Clone)]
pub struct RsaPrivKey(RsaPrivateKey);
impl RsaPrivKey {
    pub fn new() -> ResultType<Self> {
        let mut rng = rand::rngs::OsRng;
        let bits = 1024;
        let priv_key = RsaPrivateKey::new(&mut rng, bits)?;
        Ok(RsaPrivKey(priv_key))
    }
    pub fn priv_key_decrypt(&self, data: &[u8]) -> ResultType<Vec<u8>> {
        let dec_data = self.0.decrypt(PaddingScheme::PKCS1v15Encrypt, data)?;
        Ok(dec_data)
    }
    pub fn priv_key_sign(&self, data: &[u8]) -> ResultType<Vec<u8>> {
        let sign_data = self.0.sign(PaddingScheme::new_pkcs1v15_sign(None), data)?;
        Ok(sign_data)
    }
    pub fn to_public_key(&self) -> ResultType<Vec<u8>> {
        let pub_key = RsaPublicKey::from(&self.0);
        Ok(pub_key.to_pkcs1_der()?.as_ref().to_vec())
    }
}
#[derive(Debug, Clone)]
pub struct RsaPubKey(RsaPublicKey);

impl RsaPubKey {
    pub fn new(data: Vec<u8>) -> ResultType<Self> {
        let pub_key = RsaPublicKey::from_pkcs1_der(&data)?;
        Ok(RsaPubKey(pub_key))
    }

    pub fn pub_key_encrypt(&self, data: &[u8]) -> ResultType<Vec<u8>> {
        let enc_data =
            self.0
                .encrypt(&mut rand::rngs::OsRng, PaddingScheme::PKCS1v15Encrypt, data)?;
        Ok(enc_data)
    }
    pub fn pub_key_verify(&self, data: &[u8], sign: &[u8]) -> ResultType<()> {
        Ok(self
            .0
            .verify(PaddingScheme::new_pkcs1v15_sign(None), data, sign)?)
    }
}
