use std::{
    thread,
    time::{Duration, Instant},
};

use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use stream::{
    message::ChannelType,
    relay::RelayServer,
    remote_channel::{ChannelManager, ChannelReceiver, ChannelSender},
    tokio,
    utils::RsaPrivKey,
    ResultType,
};
// use crate::x264_utils::*;
pub fn start(sender: ChannelSender) -> ResultType<()> {

    // let display = Display::primary().unwrap();
    // let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    // let (width, height) = (capturer.width(), capturer.height());

    // let mut par = Param::default_preset("superfast", "zerolatency").unwrap();

    // par = par.set_dimension(height, width);
    // // par = par.param_parse("repeat_headers", "1").unwrap();
    // // par = par.param_parse("annexb", "1").unwrap();
    // // par = par.apply_profile("high").unwrap();
    // let mut pic = Picture::from_param(&par).unwrap();
    // let mut enc = Encoder::open(&mut par).unwrap();

    // let one_second = Duration::new(1, 0);
    // let fps = 60;
    // let one_frame = one_second / fps;
    // let mut timestamp = 0;
    // let r = pic.as_mut();
    // log::info!("开始编码");
    // loop {

    //     let now = Instant::now();
    //     let frame = match capturer.frame() {
    //         Ok(buffer) => buffer,
    //         Err(error) => {
    //             if error.kind() == WouldBlock {
    //                 continue;
    //             } else {
    //                 panic!("Error: {}", error);
    //             }
    //         }
    //     };
    //     convert::convert::bgra_to_i420(width, height, &frame,r[0],r[1],r[2]);
    //     pic = pic.set_timestamp(timestamp);
    //     if let Ok(Some((nal, _, _))) = enc.encode(&pic) {
    //         let buf = nal.vec;
    //         println!("encode {:?}",buf.len());

    //         sender.send(buf).await?;

    //     }
    //     println!("encode time:{:?}",now.elapsed());
    //     let elp = now.elapsed();
    //     if elp < one_frame {
    //         let sleep1 = one_frame - now.elapsed();
    //         thread::sleep(sleep1);
    //     }
    //     timestamp += 1;
    //     // break;
    // }
    // log::info!("视频通道停止");
    Ok(())
}
