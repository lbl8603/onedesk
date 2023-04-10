use stream::{
    message::ChannelType,
    relay::RelayServer,
    remote_channel::{ChannelManager, ChannelReceiver, ChannelSender},
    ResultType,
};

use crate::{
    config::Config,
    server::{key_event_server, video_server},
};

use super::mouse_event_server;


pub fn control_server_callback(
    peer_id: String,
    relay_id: String,
    relay_addr: String,
    relay_pub_key: Vec<u8>,
) {
    log::info!("服务端回调：{:?}", peer_id);
    match control_server_callback_(peer_id, relay_id, relay_addr, relay_pub_key) {
        Ok(_) => {}
        Err(e) => {
            log::info!("control_server_callback:{:?}", e);
        }
    };
}

fn control_server_callback_(
    peer_id: String,
    relay_id: String,
    relay_addr: String,
    relay_pub_key: Vec<u8>,
) -> ResultType<()> {
    let my_id = Config::get_id();
    let my_password = &Config::get_password_hash();
    let my_priv_key = Config::get_priv();
    let framed = RelayServer::start(
        my_id,
        my_password,
        &my_priv_key,
        relay_addr,
        relay_id,
        peer_id,
        relay_pub_key,
    )
    ?;
    log::info!("服务端建立通道");
    //服务端啥也不管
    let _channel_manager = ChannelManager::new_relay(
        false,
        framed,
        create_channel_callback,
        destroy_channel_callback,
    )?;

    Ok(())
}
pub fn create_channel_callback(
    sender: Option<ChannelSender>,
    receiver: Option<ChannelReceiver>,
    channel_type: ChannelType,
) {
    log::info!("创建通道：{:?},sender:{:?}", channel_type, sender);
    match channel_type {
        ChannelType::NoDefine => {}
        ChannelType::Video => {
            // std::thread::spawn(move || {
            //     TOKIO_RUNTIME.block_on(async move {
            //         let r = video_server::start(sender.unwrap()).await ;
            //         log::info!("视频通道:{:?}",r);
            //       });

            // });
        }
        ChannelType::KeyEvent => {
            let rs = key_event_server::start(receiver.unwrap());
            log::info!("创建通道：{:?},rs:{:?}", channel_type, rs);
        }
        ChannelType::MouseEvent => {
            let rs =mouse_event_server::start(receiver.unwrap());
            log::info!("创建通道：{:?},rs:{:?}", channel_type, rs);
        }
    }
}
pub fn destroy_channel_callback(channel_id: u32, channel_type: ChannelType) {
    log::info!("销毁通道：{:?},id:{}", channel_type, channel_id)
}
