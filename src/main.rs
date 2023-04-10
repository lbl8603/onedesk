use std::time;

use enigo::KeyboardControllable;
use stream::{
    id_conn::IdClient,
    rand::{self, Rng},
    tokio::{self, sync::mpsc::channel},
    utils::{self, Aes128GcmUtil},
    RemoteError,
};

use crate::{config::Config, ui::{
    index::IndexWindow,
    remote::{RemoteEventHandler, RemoteWindow},
}};

// pub mod x264_utils;
pub mod client;
pub mod config;
pub mod input_utils;
pub mod public;
pub mod server;
pub mod ui;

#[tokio::main]
async fn main() {
    let _ = log4rs::init_file("./log.yaml", Default::default());
    let user_id = Config::get_id();
    let server_key = "123".to_string();
    let cert = stream::utils::Cert::new().unwrap();
    let error_callback = |str| {
        println!("error_callback:{:?}", str);
    };
    let priv_key = Config::get_priv();
    let pub_key = priv_key.to_public_key().unwrap();
    let client = IdClient::new(
        user_id,
        server_key,
        cert,
        pub_key,
        error_callback,
        server::remote_event_server::control_server_callback,
        client::remote_event_client::control_client_callback,
    );
    let (s, r) = channel(10);
    let r_a = match client.start("localhost:8080", "localhost", r).await {
        Ok(a) => a,
        Err(e) => {
            let d = e.downcast_ref::<RemoteError>();
            println!("{:?}", d);
            return;
        }
    };
    let _take = tokio::spawn(async move{
        let r =r_a.await;
        println!("{:?}",r);
    });
    let mut frame = sciter::WindowBuilder::main_window()
        .with_pos((300, 400))
        .with_size((786, 524))
        .create();
    frame.event_handler(IndexWindow::new(s));

    frame.load_html(include_bytes!("ui/index.html"), Some("ui/index.html"));
    frame.run_app();

    // let now = time::Instant::now();
    // let runtime = stream::tokio::runtime::Builder::new_current_thread()
    // .enable_all()
    // .build()
    // .unwrap();
    // println!("runtime time:{:?}",now.elapsed());
    // let now = time::Instant::now();
    // runtime.block_on(async{
    //     println!("runtime time:{:?}","test");
    // });
    // println!("runtime time:{:?}",now.elapsed());

    //
    // let mut frame = sciter::WindowBuilder::main_window()
    //     .with_pos((300, 400))
    //     .with_size((786, 524))
    //     .create();
    // frame.event_handler(RemoteEventHandler::new());

    // // frame.register_behavior("video-generator", || Box::new(RemoteWindow::new()));
    // frame.load_html(include_bytes!("ui/remote.html"), Some("ui/remote.html"));
    // frame.run_app();
    // println!("encode time:{:?}",now.elapsed());
    // println!("加密：{:?}",data);
    // let now = time::Instant::now();
    // let data_d = aes.decrypt(&data).unwrap();
    // println!("decrypt time:{:?}",now.elapsed());
    // println!("解密{:?}",data_d);
    // enigo.mo
    // enigo.key_down(key)
    // log4rs::init_file("./log.yaml", Default::default()).unwrap();

    // let user_id = "123".to_string();
    // let server_key = "123".to_string();
    // let cert = stream::utils::Cert::new().unwrap();
    // let error_callback = |str|{
    //     println!("error_callback:{:?}",str);
    // };
    // let control_server_callback = |
    //     peer_id: String,
    //     relay_id: String,
    //     relay_addr: String,
    //     relay_pub_key: Vec<u8>
    // |{
    //     println!("control_server_callback:{:?}",peer_id);
    // };
    // let control_client_callback = |  peer_id: String,
    // peer_pub_key: Vec<u8>,
    // relay_id: String,
    // relay_addr: String,
    // relay_pub_key: Vec<u8>|{
    //     println!("control_client_callback:{:?}",peer_id);
    // };
    // let priv_key = stream::utils::RsaPrivKey::new().unwrap();
    // let pub_key = priv_key.to_public_key().unwrap();
    // let mut client = IdClient::new(user_id, server_key, cert,pub_key ,error_callback, control_server_callback, control_client_callback);
    // let (s,r) = channel(10);
    // let r_a = match client.start("localhost:8080", "localhost",r).await {
    //     Ok(a) => {a},
    //     Err(e) => {
    //         let d = e.downcast_ref::<RemoteError>();
    //         println!("{:?}",d);
    //         return;
    //     },
    // };

    // let take = tokio::spawn(async move{
    //     let r =r_a.await;
    //     println!("{:?}",r);
    // });

    // let start = time::Instant::now();
    // let mut request = stream::rendezvous_proto::RendezvousMessage::new();
    // request.set_request_relay(stream::rendezvous_proto::RequestRelay{
    //     peer_id:String::from("123"),
    //     ..Default::default()
    // });
    // s.send(stream::protobuf::Message::write_to_bytes(&request).unwrap()).await.unwrap();
    // println!("pub_k:{:?}",start.elapsed());
    // take.await.unwrap();

    // stream::tcp::TcpServer::new(&utils::pkcs12().unwrap());
    // let start = time::Instant::now();
    // let rsa = stream::utils::RsaPrivKey::new().unwrap();
    // println!("生成:{:?}",start.elapsed());
    // let data = b"hello world";
    // let start = time::Instant::now();
    // let pub_k = rsa.to_public_key().unwrap();
    // println!("pub_k:{:?}",start.elapsed());
    // let pub_key = stream::utils::RsaPubKey::new(pub_k).unwrap();
    // println!("pub_k 2:{:?}",start.elapsed());
    // let d = pub_key.pub_key_encrypt(data).unwrap();
    // println!("加密:{:?}",start.elapsed());
    // println!("{:?}", &d);

    // let src_data = rsa.priv_key_decrypt(&d).unwrap();
    // println!("解密:{:?}",start.elapsed());
    // println!("{:?}", &src_data);
    // let start = time::Instant::now();
    // let sign = rsa.priv_key_sign(data).unwrap();

    // println!("签名:{:?}",start.elapsed());
    // let data = b"hello world";
    // println!("{:?}", &sign);
    // let ver = pub_key.pub_key_verify(data,&sign).unwrap();
    // println!("验证:{:?}",start.elapsed());
    // println!("Hello, world!");
}
