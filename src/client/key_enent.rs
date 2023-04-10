use crossbeam::atomic::AtomicCell;
use stream::message::{key_event, KeyEvent};
use stream::protobuf::Message;
use stream::remote_channel::ChannelSender;
use stream::{protobuf, ResultType};
use winapi::ctypes::c_int;
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HHOOK;
use winapi::um::winuser;

use std::convert::TryFrom;
use std::sync::atomic::AtomicBool;

lazy_static::lazy_static! {
    static ref RUN_STATUS:AtomicBool = AtomicBool::new(false);
    static ref SENDER_STATUS:AtomicBool = AtomicBool::new(false);
    static ref SENDER_CELL:AtomicCell<Option<ChannelSender>> =  AtomicCell::new(None);
    static ref IS_HOOK:AtomicBool = AtomicBool::new(false);
}
pub fn hook(is_hook: bool) {
    IS_HOOK.store(is_hook, std::sync::atomic::Ordering::SeqCst)
}
pub fn is_run() -> bool {
    RUN_STATUS.load(std::sync::atomic::Ordering::SeqCst)
        && SENDER_STATUS.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn start(sender: ChannelSender) -> ResultType<()> {
    SENDER_CELL.store(Some(sender));
    SENDER_STATUS.store(true, std::sync::atomic::Ordering::SeqCst);
    let run_status = RUN_STATUS.load(std::sync::atomic::Ordering::SeqCst);

    if !run_status {
        RUN_STATUS.store(true, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(|| {
            let hook = setup_hook();
            message_loop();
            remove_hook(hook);
            RUN_STATUS.store(false, std::sync::atomic::Ordering::SeqCst);
        });
    }
    Ok(())
}

fn setup_hook() -> HHOOK {
    unsafe {
        let hook = winuser::SetWindowsHookExA(
            winuser::WH_KEYBOARD_LL,
            Some(callback),
            std::ptr::null_mut(),
            0,
        );

        if hook.is_null() {
            panic!("Windows hook null return");
        }

        println!("Successfully hooked keyboard");

        hook
    }
}

fn remove_hook(hook: HHOOK) {
    unsafe {
        let result = winuser::UnhookWindowsHookEx(hook);

        if result == 0 {
            panic!("Windows unhook non-zero return");
        }

        println!("Successfully unhooked keyboard");
    }
}

fn message_loop() {
    // This function handles the event loop, which is necessary for the hook to function
    let mut msg = winuser::MSG::default();
    unsafe {
        while 0 == winuser::GetMessageA(&mut msg, std::ptr::null_mut(), 0, 0) {
            winuser::TranslateMessage(&msg);
            winuser::DispatchMessageA(&msg);
        }
    }
}

#[allow(dead_code)]
unsafe extern "system" fn callback(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let is_hook = IS_HOOK.load(std::sync::atomic::Ordering::SeqCst);
    let is_send = SENDER_STATUS.load(std::sync::atomic::Ordering::SeqCst);
    if is_hook && is_send && code == winuser::HC_ACTION {
        let data = match UINT::try_from(w_param).unwrap() {
            winuser::WM_KEYDOWN | winuser::WM_SYSKEYDOWN => {
                let info: winuser::PKBDLLHOOKSTRUCT = std::mem::transmute(l_param);
                let key_data = KeyEvent {
                    key: (*info).vkCode,
                    active: protobuf::ProtobufEnumOrUnknown::from(key_event::Active::Down),
                    ..Default::default()
                };
                key_data.write_to_bytes().unwrap()
                // match key_data.write_to_bytes() {
                //     Ok(data) => {

                //     }
                //     Err(err) => {
                //         log::info!("键盘数据序列化失败:{:?}", err);
                //     }
                // }

                // println!("Keydown: {} scanCode:{},time:{}", (*info).vkCode, (*info).scanCode,(*info).time);
            }

            winuser::WM_KEYUP | winuser::WM_SYSKEYUP => {
                let info: winuser::PKBDLLHOOKSTRUCT = std::mem::transmute(l_param);
                let key_data = KeyEvent {
                    key: (*info).vkCode,
                    active: protobuf::ProtobufEnumOrUnknown::from(key_event::Active::Up),
                    ..Default::default()
                };
                key_data.write_to_bytes().unwrap()
                // match key_data.write_to_bytes() {
                //     Ok(data) => {
                //         if let Some(sender) = SENDER_CELL.take() {
                //             if let Err(e) = TOKIO_RUNTIME.block_on(sender.send(data)) {
                //                 SENDER_STATUS.store(false, std::sync::atomic::Ordering::SeqCst);
                //                 log::info!("发送键盘事件失败:{:?}", e);
                //             } else {
                //                 SENDER_CELL.store(Some(sender));
                //             }
                //         }
                //     }
                //     Err(err) => {
                //         log::info!("键盘数据序列化失败:{:?}", err);
                //     }
                // }

                // println!("Keyup: {}", (*info).vkCode);
            }

            _ => return 1,
        };
        if let Some(mut sender) = SENDER_CELL.take() {
            if let Err(e) = sender.send(data) {
                SENDER_STATUS.store(false, std::sync::atomic::Ordering::SeqCst);
                log::info!("发送键盘事件失败:{:?}", e);
            } else {
                SENDER_CELL.store(Some(sender));
            }
        }
        return 1;
    }
    winuser::CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param);
    0
    //
}
