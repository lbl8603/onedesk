
use std::sync::RwLock;

use directories_next::ProjectDirs;
use serde_derive::{Deserialize, Serialize};
use stream::sha2::Digest;
use stream::{rand::Rng, utils::RsaPrivKey};
const APPNAME: &str = "onedesk";
const CHARS: &'static [char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

lazy_static::lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(Config::load());
    static ref PRIV:RsaPrivKey =  RsaPrivKey::new().unwrap();
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Config {
    id: String,
    password: String,
}

impl Config {
    fn load() -> Self {
        if let Some(project) = ProjectDirs::from("", "", APPNAME) {
            let mut path = project.data_local_dir().to_path_buf();
            path.push("user.yaml");
            println!("{:?}", path);
            if let Ok(conf) = confy::load_path(path) {
                return conf;
            }
        }
        Default::default()
    }
    fn store(&self) {
        if let Some(project) = ProjectDirs::from("", "", APPNAME) {
            let mut path = project.data_local_dir().to_path_buf();
            path.push("user.yaml");

            if let Err(err) = confy::store_path(path, self) {
                log::error!("Failed to store config: {}", err);
            }
        }
    }
    pub fn get_priv() -> RsaPrivKey {
        PRIV.clone()
    }
    pub fn get_id() -> String {
        String::from("12345")
        // CONFIG.read().unwrap().id.clone()
    }
    pub fn update_id() -> String {
        let rand1: u64 = stream::rand::thread_rng().gen();
        let id = rand1.to_string()[..6].to_string();
        Config::set_id(id.clone());
        id
    }
    fn set_id(id: String) {
        let mut w = CONFIG.write().unwrap();
        w.id = id;
        w.store();
    }
    pub fn get_password() -> String {
        CONFIG.read().unwrap().password.clone()
    }
    pub fn get_password_hash() -> Vec<u8> {
        let password = CONFIG.read().unwrap().password.clone();
        let mut hasher2 = stream::sha2::Sha256::new();
        hasher2.update(password);
        hasher2.finalize()[..].into()
    }

    pub fn update_password() -> String {
        let mut rng = stream::rand::thread_rng();
        let rs: String = (0..6)
            .map(|_| CHARS[rng.gen::<usize>() % CHARS.len()])
            .collect();
        Config::set_password(rs.clone());
        rs
    }
    fn set_password(password: String) {
        let mut w = CONFIG.write().unwrap();
        w.password = password;
        w.store();
    }
}
