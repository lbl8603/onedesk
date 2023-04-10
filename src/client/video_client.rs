use std::{ptr::null_mut, sync::atomic::AtomicU64, time::Instant};

use openh264_sys::{
    ISVCDecoderVtbl, SBufferInfo, SDecodingParam, WelsCreateDecoder, WelsDestroyDecoder,
};
use sciter::video::{video_destination, AssetPtr, COLOR_SPACE};
use stream::{remote_channel::ChannelReceiver, ResultType};

lazy_static::lazy_static! {
    static ref FPS:AtomicU64 = AtomicU64::new(0);
}

pub  fn start(
    site: &mut AssetPtr<video_destination>,
    receiver: ChannelReceiver,
) -> ResultType<String> {
    let decoder = null_mut();
    let rs = start_( site, receiver, decoder);
    unsafe {
        WelsDestroyDecoder(decoder);
    }
    rs
}
pub fn fps() -> u64 {
    FPS.load(std::sync::atomic::Ordering::SeqCst)
}
 fn start_(
    site: &mut AssetPtr<video_destination>,
    receiver: ChannelReceiver,
    mut decoder: *mut *const ISVCDecoderVtbl,
) -> ResultType<String> {
    let param = SDecodingParam::default();
    let mut height = 0;
    let mut width = 0;
    unsafe {
        if WelsCreateDecoder(&mut decoder) != 0 || decoder.is_null() {
            return Ok(String::from("创建解码器失败"));
        }
        if (**decoder).Initialize.unwrap()(decoder, &param) != 0 {
            return Ok(String::from("初始化解码器失败"));
        }
        // assert_eq!(WelsCreateDecoder(&mut decoder), 0);
        // assert!(!decoder.is_null());
        // assert_eq!((**decoder).Initialize.unwrap()(decoder, &param), 0);

        let mut buf = SBufferInfo::default();

        let mut dst = [null_mut(); 3];
        if let Some(decoder_fn) = (**decoder).DecodeFrameNoDelay {
            loop {
                let now = Instant::now();
                let data = if let Ok(data) = receiver.recv() {
                    data
                } else {
                    return Ok(String::from("视频数据接收失败"));
                };
                let dd = decoder_fn(
                    decoder,
                    data.as_ptr(),
                    data.len() as i32,
                    &mut dst as *mut _,
                    &mut buf,
                );
                if dd != 0 || buf.iBufferStatus != 1 {
                    return Ok(String::from("视频解码失败"));
                }

                let info = buf.UsrData.sSystemBuffer;

                if height != info.iHeight || width != info.iStride[0] {
                    height = info.iHeight;
                    width = info.iStride[0];
                    if let Err(_) =
                        site.start_streaming((width as i32, height as i32), COLOR_SPACE::Iyuv, None)
                    {
                        return Ok(String::from("视频播放初始化失败"));
                    }
                }

                // https://github.com/cisco/openh264/issues/2379
                let y: &[u8] =
                    std::slice::from_raw_parts(dst[0], (info.iHeight * info.iStride[0]) as usize);
                let u: &[u8] = std::slice::from_raw_parts(
                    dst[1],
                    (info.iHeight * info.iStride[1] / 2) as usize,
                );
                let v: &[u8] = std::slice::from_raw_parts(
                    dst[2],
                    (info.iHeight * info.iStride[1] / 2) as usize,
                );
                let mut add = Vec::new();
                add.extend_from_slice(y);
                add.extend_from_slice(u);
                add.extend_from_slice(v);
                if let Err(_) = site.render_frame(&add) {
                    return Ok(String::from("视频数据播放失败"));
                }
                FPS.store(
                    1000 / (now.elapsed().as_millis() as u64),
                    std::sync::atomic::Ordering::SeqCst,
                );
                // log::info!("总延迟：{:?}",now.elapsed());
            }
        }
        return Ok(String::from("获取解码器方法失败"));
    }
}
