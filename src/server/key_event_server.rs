use enigo::{Key, KeyboardControllable};
use stream::{message::KeyEvent, protobuf::Message, remote_channel::ChannelReceiver, ResultType};

pub  fn start(receiver: ChannelReceiver) -> ResultType<()> {
    let mut enigo = enigo::Enigo::new();
    while let Ok(data) = receiver.recv() {
        let key_data = KeyEvent::parse_from_bytes(&data)?;
        match key_data.active.enum_value_or_default() {
            //键盘事件用钩子获取到的值能直接用，不需要额外转换
            stream::message::key_event::Active::Click => {
                enigo.key_click(Key::Raw(key_data.key as u16));
            }
            stream::message::key_event::Active::Down => {
                enigo.key_down(Key::Raw(key_data.key as u16));
            }
            stream::message::key_event::Active::Up => {
                enigo.key_up(Key::Raw(key_data.key as u16));
            }
        }
    }
    log::info!("键盘通道断开");
    Ok(())
}
