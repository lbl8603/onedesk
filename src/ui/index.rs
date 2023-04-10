use sciter::dom::event::*;
use sciter::{Element, HELEMENT};
use stream::protobuf::Message;

use crate::config::Config;

pub struct IndexWindow {
    sender: stream::tokio::sync::mpsc::Sender<Vec<u8>>,
}

impl Drop for IndexWindow {
    fn drop(&mut self) {
        println!("[video] behavior is destroyed");
    }
}
use crate::public::TOKIO_RUNTIME;
impl IndexWindow {
    pub fn new(sender: stream::tokio::sync::mpsc::Sender<Vec<u8>>) -> Self {
        Self { sender }
    }
    fn print(&self, key: String) {
        println!("key:{}", key);
    }
    fn get_id(&self) -> String {
        Config::get_id()
    }
    fn get_password(&self) -> String {
        Config::get_password()
    }
    fn update_id(&self) -> String {
        Config::update_id()
    }
    fn update_password(&self) -> String {
        Config::update_password()
    }
    fn connect(&self, peer_id: String) {
        println!("peer:{}", peer_id);
        let my_id = Config::get_id();
        if my_id != peer_id {
            let mut request = stream::rendezvous_proto::RendezvousMessage::new();
            request.set_request_relay(stream::rendezvous_proto::RequestRelay {
                peer_id,
                ..Default::default()
            });
            let sender = self.sender.clone();
            std::thread::spawn(move || {
                let rs = TOKIO_RUNTIME.block_on(sender.send(request.write_to_bytes().unwrap()));
                log::info!("rs:{:?}",rs);
            });
           
        }
    }
}

impl sciter::EventHandler for IndexWindow {
    sciter::dispatch_script_call! {
      fn print(String);
      fn get_id();
      fn get_password();
      fn update_id();
      fn update_password();
      fn connect(String);
    }
    fn get_subscription(&mut self) -> Option<EVENT_GROUPS> {
        Some(EVENT_GROUPS::HANDLE_BEHAVIOR_EVENT)
    }

    fn detached(&mut self, _root: HELEMENT) {
        println!("[video] <video> element is detached");
    }
}
