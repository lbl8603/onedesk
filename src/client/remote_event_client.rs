use sciter::video::{video_destination, AssetPtr};
use stream::{
    message::ChannelType,
    relay::{LoginResponseEnum, RelayClient},
    remote_channel::{ChannelManager, ChannelReceiver, ChannelSender},
    ResultType,
};

use crate::ui::remote::{RemoteEventHandler, RemoteWindow};

use super::{key_enent, mouse_event, video_client};

pub fn control_client_callback(
    peer_id: String,
    peer_pub_key: Vec<u8>,
    relay_id: String,
    relay_addr: String,
    relay_pub_key: Vec<u8>,
) {
    log::info!("客户端回调：{:?}", peer_id);
    std::thread::spawn(move || {
        let mut frame = sciter::WindowBuilder::main_window()
            .with_pos((300, 400))
            .with_size((786, 524))
            .create();
        frame.event_handler(RemoteEventHandler::new());
        frame.register_behavior("video-generator", move || {
            Box::new(RemoteWindow::new(
                peer_id.clone(),
                peer_pub_key.clone(),
                relay_id.clone(),
                relay_addr.clone(),
                relay_pub_key.clone(),
            ))
        });
        frame.load_html(
            include_bytes!("../ui/remote.html"),
            Some("../ui/remote.html"),
        );
        frame.run_app();
        log::info!("远程窗口关闭");
    });
}

pub fn control_client<F>(
    password_callback: F,
    mut site: AssetPtr<video_destination>,
    my_id: String,
    peer_id: String,
    peer_pub_key: Vec<u8>,
    relay_id: String,
    relay_addr: String,
    relay_pub_key: Vec<u8>,
) -> ResultType<ChannelManager>
where
    F: Fn(String, String, LoginResponseEnum) -> Option<Vec<u8>> + 'static,
{
    log::info!("客户端通道");
    let framed = RelayClient::start(
        my_id,
        password_callback,
        relay_addr,
        relay_id,
        peer_id,
        peer_pub_key,
        relay_pub_key,
    )?;
    let mut channel_manager = ChannelManager::new_relay(
        true,
        framed,
        create_channel_callback,
        destroy_channel_callback,
    )?;

    log::info!("转发连接建立成功");
    //视频通道
    let video = channel_manager.create_channel_read(ChannelType::Video)?;
    video_client::start(&mut site, video)?;
    let key_channel = channel_manager.create_channel_write(ChannelType::KeyEvent)?;
    key_enent::start(key_channel)?;
    let mouse_channel = channel_manager.create_channel_write(ChannelType::MouseEvent)?;
    mouse_event::load(mouse_channel);
    Ok(channel_manager)
}
pub fn create_channel_callback(
    sender: Option<ChannelSender>,
    _receiver: Option<ChannelReceiver>,
    channel_type: ChannelType,
) {
    log::info!("创建客户端通道：{:?},sender:{:?}", channel_type, sender);
}
pub fn destroy_channel_callback(channel_id: u32, channel_type: ChannelType) {
    log::info!("销毁客户端通道：{:?},id:{}", channel_type, channel_id)
}
