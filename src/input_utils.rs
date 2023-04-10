use enigo::MouseButton;

pub fn to_mouse(key: u32) -> MouseButton {
    match key {
        1 => MouseButton::Left,
        2 => MouseButton::Middle,
        3 => MouseButton::Right,
        _ => MouseButton::Left,
    }
}
// fn to_key(_key: u32) -> Key {
//     let k = Key::from(1);
// }
