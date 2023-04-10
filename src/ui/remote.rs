

use crate::client::{key_enent, mouse_event};
use crate::config::Config;
use crossbeam::atomic::AtomicCell;
use sciter::dom::event::*;
use sciter::video::{video_destination, AssetPtr};
use sciter::{make_args, Element, HELEMENT};
use stream::sha2::{Digest, Sha256};
lazy_static::lazy_static! {
  static ref ELEMENT_CELL:AtomicCell<Option<Element>> =  AtomicCell::new(None);
}
pub struct RemoteEventHandler;
impl RemoteEventHandler {
    pub fn new() -> Self {
        Self {}
    }

    fn print(&self, key: String) {
        println!("key:{}", key);
    }
    fn get_id(&self) -> String {
        Config::get_id()
    }
    fn mouse_enter(&self) {
        key_enent::hook(true);
    }
    fn mouse_leave(&self) {
        key_enent::hook(false);
    }
    fn mouse_event(&self) {}
    // #[inline]
    // fn call(&self, func: &str, args: &[Value]) {
    //     if let Some(e) = ELEMENT_CELL.take() {
    //         let rs = e.call_method(func, args);
    //         ELEMENT_CELL.store(Some(e));
    //         log::info!("call:{:?},func:{},args:{:?}", rs, func, args);
    //     } else {
    //         log::info!("call is none");
    //     }
    // }
}
impl sciter::EventHandler for RemoteEventHandler {
    sciter::dispatch_script_call! {
      fn print(String);
      fn get_id();
      fn mouse_leave();
      fn mouse_enter();
    }
}
pub struct RemoteWindow {
    // element: Option<Element>,
    video: Option<AssetPtr<video_destination>>,
    peer_id: Option<String>,
    peer_pub_key: Option<Vec<u8>>,
    relay_id: Option<String>,
    relay_addr: Option<String>,
    relay_pub_key: Option<Vec<u8>>,
    channel_manager: Option<stream::remote_channel::ChannelManager>,
}
pub struct CallHandler(Element);
impl CallHandler {
    //弹窗并获取密码
    pub fn passwrod_wind(&self, peer_id: String, msg: String) -> Option<Vec<u8>> {
        if let Ok(rs) = self.0.call_method("getPassword", &make_args!(peer_id, msg)) {
            if let Some(password) = rs.as_string() {
                let mut hasher2 = Sha256::new();
                hasher2.update(password);
                let d = hasher2.finalize()[..].into();
                log::info!("call:success,func:getPassword ");
                return Some(d);
            }
        }
        log::info!("获取密码失败");
        None
    }
}

impl Drop for RemoteWindow {
    fn drop(&mut self) {
        println!("[video] behavior is destroyed");
    }
}

impl RemoteWindow {
    pub fn new(
        peer_id: String,
        peer_pub_key: Vec<u8>,
        relay_id: String,
        relay_addr: String,
        relay_pub_key: Vec<u8>,
    ) -> Self {
        Self {
            // element: None,
            video: Default::default(),
            peer_id: Some(peer_id),
            peer_pub_key: Some(peer_pub_key),
            relay_id: Some(relay_id),
            relay_addr: Some(relay_addr),
            relay_pub_key: Some(relay_pub_key),
            channel_manager: None,
        }
    }
}

impl sciter::EventHandler for RemoteWindow {
    fn get_subscription(&mut self) -> Option<EVENT_GROUPS> {
        Some(EVENT_GROUPS::HANDLE_BEHAVIOR_EVENT)
    }
    fn attached(&mut self, root: HELEMENT) {
        println!("[video] <video> element is attached");
        // self.element = Some(Element::from(root));
        if let Some(site) = self.video.take() {
            println!("开始连接");
            let hall_handler = CallHandler(Element::from(root));
            let password_callback =
                move |peer_id: String,
                      _check_info: String,
                      rs: stream::relay::LoginResponseEnum| {
                    match rs {
                        stream::relay::LoginResponseEnum::First => {
                            hall_handler.passwrod_wind(peer_id, String::new())
                        }
                        stream::relay::LoginResponseEnum::NotMatch => {
                            hall_handler.passwrod_wind(peer_id, String::from("密码错误"))
                        }
                        stream::relay::LoginResponseEnum::Frequently => {
                            hall_handler.passwrod_wind(peer_id, String::from("错误次数过多"))
                        }
                    }
                };
            let peer_id = self.peer_id.take().unwrap();
            let peer_pub_key = self.peer_pub_key.take().unwrap();
            let relay_id = self.relay_id.take().unwrap();
            let relay_addr = self.relay_addr.take().unwrap();
            let relay_pub_key = self.relay_pub_key.take().unwrap();
            let my_id = Config::get_id();
            match crate::client::remote_event_client::control_client(
                password_callback,
                site,
                my_id,
                peer_id,
                peer_pub_key,
                relay_id,
                relay_addr,
                relay_pub_key,
            ) {
                Ok(channel_manager) => self.channel_manager = Some(channel_manager),
                Err(e) => {
                    log::info!("连接错误：{:?}", e);
                }
            };
        }
    }
    fn detached(&mut self, _root: HELEMENT) {
        println!("[video] <video> element is detached");
    }

    fn on_event(
        &mut self,
        _root: HELEMENT,
        source: HELEMENT,
        _target: HELEMENT,
        code: BEHAVIOR_EVENTS,
        phase: PHASE_MASK,
        reason: EventReason,
    ) -> bool {
        if phase != PHASE_MASK::BUBBLING {
            return false;
        }

        match code {
            BEHAVIOR_EVENTS::VIDEO_BIND_RQ => {
                // let source = Element::from(source);
                if let EventReason::VideoBind(ptr) = reason {
                    if ptr.is_null() {
                        return true;
                    }

                    let site = AssetPtr::adopt(ptr as *mut video_destination);
                    println!("开始连接");
                    let hall_handler = CallHandler(Element::from(source));
                    let password_callback =
                        move |peer_id: String,
                              _check_info: String,
                              rs: stream::relay::LoginResponseEnum| {
                            println!("弹窗:{:?}", rs);
                            match rs {
                                stream::relay::LoginResponseEnum::First => {
                                    hall_handler.passwrod_wind(peer_id, String::new())
                                }
                                stream::relay::LoginResponseEnum::NotMatch => {
                                    hall_handler.passwrod_wind(peer_id, String::from("密码错误"))
                                }
                                stream::relay::LoginResponseEnum::Frequently => hall_handler
                                    .passwrod_wind(peer_id, String::from("错误次数过多")),
                            }
                        };
                    let peer_id = self.peer_id.take().unwrap();
                    let peer_pub_key = self.peer_pub_key.take().unwrap();
                    let relay_id = self.relay_id.take().unwrap();
                    let relay_addr = self.relay_addr.take().unwrap();
                    let relay_pub_key = self.relay_pub_key.take().unwrap();
                    let my_id = Config::get_id();
                    //启动线程
                    match crate::client::remote_event_client::control_client(
                        password_callback,
                        site,
                        my_id,
                        peer_id,
                        peer_pub_key,
                        relay_id,
                        relay_addr,
                        relay_pub_key,
                    ) {
                        Ok(channel_manager) => self.channel_manager = Some(channel_manager),
                        Err(e) => {
                            log::info!("连接错误：{:?}", e);
                        }
                    };
                    // self.video.store(Some(site));
                    // println!("start [video] {:?} {} ({:?})", code, source, reason);
                }
            }

            BEHAVIOR_EVENTS::VIDEO_INITIALIZED => {
                println!("[video] {:?}", code);
            }

            BEHAVIOR_EVENTS::VIDEO_STARTED => {
                println!("[video] {:?}", code);

                let source = Element::from(source);
                use sciter::dom::ELEMENT_AREAS;
                let flags = ELEMENT_AREAS::CONTENT_BOX as u32 | ELEMENT_AREAS::SELF_RELATIVE as u32;
                let rc = source.get_location(flags).unwrap();
                println!(
                    "[video] start video thread on <{}> which is about {:?} pixels",
                    source,
                    rc.size()
                );
            }

            BEHAVIOR_EVENTS::VIDEO_STOPPED => {
                println!("[video] {:?}", code);
            }

            _ => return false,
        };

        return true;
    }
}
