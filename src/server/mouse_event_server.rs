use enigo::MouseControllable;
use stream::{message::MouseEvent, protobuf::Message, remote_channel::ChannelReceiver, ResultType};

use crate::input_utils::to_mouse;

pub fn start(receiver: ChannelReceiver) -> ResultType<()> {
    let mut enigo = enigo::Enigo::new();
    while let Ok(data) = receiver.recv() {
        let mouse = MouseEvent::parse_from_bytes(&data)?;
        match mouse.active.enum_value_or_default() {
            stream::message::mouse_event::Active::Click => {
                enigo.mouse_click(to_mouse(mouse.key));
            }
            stream::message::mouse_event::Active::Down => {
                enigo.mouse_down(to_mouse(mouse.key));
            }
            stream::message::mouse_event::Active::Up => {
                enigo.mouse_up(to_mouse(mouse.key));
            }
            stream::message::mouse_event::Active::Move => {
                enigo.mouse_move_to(mouse.move_x, mouse.move_y);
            }
            stream::message::mouse_event::Active::ScrollY => {
                enigo.mouse_scroll_y(mouse.scroll_len);
            }
        }
    }
    log::info!("鼠标通道断开");
    Ok(())
}
