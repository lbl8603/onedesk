use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::AtomicBool,
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
};

use dashmap::DashMap;
use protobuf::Message;

use crate::{RemoteError, ResultType, bytes_codec::{TcpFramed }, message::{ChannelControl, ChannelMover, ChannelPower, ChannelType}};

pub struct ChannelManager {
    id: u32,
    sender_in: SyncSender<Vec<u8>>,
    channel_map: Arc<DashMap<u32, SyncSender<Vec<u8>>>>,
    status: Arc<AtomicBool>,
}
impl Drop for ChannelManager {
    fn drop(&mut self) {
        //回收客户端通道
        if self.id & 1 == 0 {
            log::info!("回收客户端通道");
            if self.status.load(std::sync::atomic::Ordering::SeqCst) {
                match self.sender_in.send(Vec::new()) {
                    Ok(_) => {}
                    Err(e) => {
                        log::info!("通道管理器回收失败:{:?}", e);
                    }
                };
            }
        }
    }
}
///回调方法不能阻塞
impl ChannelManager {
    pub fn close(self) -> ResultType<()> {
        if self.status.load(std::sync::atomic::Ordering::SeqCst) {
            self.sender_in.send(Vec::new())?;
        }

        Ok(())
    }
    fn send(&mut self, data: Vec<u8>) -> ResultType<()> {
        self.sender_in.send(data)?;
        Ok(())
    }
    pub fn new_relay(
        is_client: bool,
        framed: TcpFramed,
        create_channel_callback: fn(Option<ChannelSender>, Option<ChannelReceiver>, ChannelType),
        destroy_channel_callback: fn(u32, ChannelType),
    ) -> ResultType<Self> {
        let channel_map: Arc<DashMap<u32, SyncSender<Vec<u8>>>> = Arc::new(DashMap::new());

        let status = Arc::new(AtomicBool::new(true));
        let (sender_in, receiver) = sync_channel::<Vec<u8>>(10);
        let mut write_stream = framed.try_clone()?;

        let status1 = status.clone();
        std::thread::spawn(move || loop {
            match receiver.recv() {
                Ok(data) => {
                    if data.len() == 0 {
                        let rs = write_stream.close();
                        log::info!("通道管理器 关闭流{:?}", rs);
                        break;
                    }
                    match write_stream.send(data) {
                        Ok(_d) => {}
                        Err(e) => {
                            log::info!("通道数据发送异常：{:?}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::info!("通道管理器接收异常：{:?}", e);
                    let rs = write_stream.close();
                    log::info!("通道管理器 关闭流{:?}", rs);
                    break;
                }
            }
            status1.store(false, std::sync::atomic::Ordering::SeqCst);
        });
        let status2 = status.clone();
        let sender_in1 = sender_in.clone();
        let channel_map_in = channel_map.clone();
        std::thread::spawn(move || {
            let rs = ChannelManager::next_loop(
                sender_in1,
                channel_map_in,
                framed,
                create_channel_callback,
                destroy_channel_callback,
            );
            log::info!("通道管理器 loop 关闭流{:?}", rs);
            status2.store(false, std::sync::atomic::Ordering::SeqCst);
        });
        //客户端建立的通道二进制0结尾，服务端建立的通道1结尾
        let id = if is_client { 0 } else { 1 };
        Ok(ChannelManager {
            id,
            sender_in,
            channel_map,
            status,
        })
    }
    fn next_loop(
        sender: SyncSender<Vec<u8>>,
        channel_map: Arc<DashMap<u32, SyncSender<Vec<u8>>>>,
        mut tcp_stream: TcpFramed,
        create_channel_callback: fn(Option<ChannelSender>, Option<ChannelReceiver>, ChannelType),
        destroy_channel_callback: fn(u32, ChannelType),
    ) -> ResultType<()> {
        loop {
            let data = tcp_stream.next()?;
            ChannelManager::next_msg(
                data,
                &sender,
                &channel_map,
                create_channel_callback,
                destroy_channel_callback,
            )?;
        }
    }
    fn next_msg(
        data: Vec<u8>,
        sender: &SyncSender<Vec<u8>>,
        channel_map: &Arc<DashMap<u32, SyncSender<Vec<u8>>>>,
        create_channel_callback: fn(Option<ChannelSender>, Option<ChannelReceiver>, ChannelType),
        destroy_channel_callback: fn(u32, ChannelType),
    ) -> ResultType<()> {
        let rs = ChannelMover::parse_from_bytes(&data)?;
        if let Ok(control) = rs.control.enum_value() {
            match control {
                ChannelControl::Create => {
                    match rs.channel_power.enum_value_or_default() {
                        ChannelPower::Both => {
                            let (sender_down, receiver_down) = sync_channel::<Vec<u8>>(10);
                            let sender_up = sender.clone();
                            channel_map.insert(rs.id, sender_down);
                            create_channel_callback(
                                Some(ChannelSender(sender_up, rs.id)),
                                Some(ChannelReceiver(receiver_down)),
                                rs.channel_type.enum_value_or_default(),
                            )
                        }
                        ChannelPower::Read => {
                            //对方只读，自己只写
                            let sender_up = sender.clone();
                            create_channel_callback(
                                Some(ChannelSender(sender_up, rs.id)),
                                None,
                                rs.channel_type.enum_value_or_default(),
                            )
                        }
                        ChannelPower::Write => {
                            //对方只写，自己只读
                            let (sender_down, receiver_down) = sync_channel::<Vec<u8>>(10);
                            channel_map.insert(rs.id, sender_down);
                            create_channel_callback(
                                None,
                                Some(ChannelReceiver(receiver_down)),
                                rs.channel_type.enum_value_or_default(),
                            )
                        }
                    }
                }
                ChannelControl::Destroy => {
                    channel_map.remove(&rs.id);
                    destroy_channel_callback(rs.id, rs.channel_type.enum_value_or_default())
                }
                ChannelControl::Data => {
                    let id = if let Some(channel) = channel_map.get(&rs.id) {
                        if let Err(e) = channel.value().send(rs.data) {
                            log::info!("发送到指定通道失败:{:?}", e);
                            //发送失败 通知对方销毁通道
                            channel_map.remove(&rs.id);
                            rs.id
                        } else {
                            return Ok(());
                        }
                    } else {
                        //通道不存在 通知对方销毁通道
                        rs.id
                    };
                    let channel_mover = ChannelMover {
                        id,
                        control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Destroy),
                        ..Default::default()
                    };
                    sender.send(channel_mover.write_to_bytes()?)?;
                }
            }
        }
        return Ok(());
    }
    pub fn is_run(&self) -> bool {
        self.status.load(std::sync::atomic::Ordering::SeqCst)
    }
    //只读
    pub fn create_channel_read(
        &mut self,
        channel_type: ChannelType,
    ) -> ResultType<ChannelReceiver> {
        if !self.status.load(std::sync::atomic::Ordering::SeqCst) {
            Err(RemoteError::Disconnection)?;
        }
        let (sender_down, receiver_down) = sync_channel::<Vec<u8>>(10);
        self.id += 2;
        self.channel_map.insert(self.id, sender_down);
        let channel = ChannelMover {
            id: self.id,
            control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Create),
            channel_type: protobuf::ProtobufEnumOrUnknown::new(channel_type),
            channel_power: protobuf::ProtobufEnumOrUnknown::new(ChannelPower::Read),
            ..Default::default()
        };
        log::info!("建立通道只读:{:?}", channel_type);
        self.send(channel.write_to_bytes()?)?;
        return Ok(ChannelReceiver(receiver_down));
    }
    //只写
    pub fn create_channel_write(&mut self, channel_type: ChannelType) -> ResultType<ChannelSender> {
        if !self.status.load(std::sync::atomic::Ordering::SeqCst) {
            Err(RemoteError::Disconnection)?;
        }
        self.id += 2;
        let channel = ChannelMover {
            id: self.id,
            control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Create),
            channel_type: protobuf::ProtobufEnumOrUnknown::new(channel_type),
            channel_power: protobuf::ProtobufEnumOrUnknown::new(ChannelPower::Write),
            ..Default::default()
        };
        let sender_up = self.sender_in.clone();
        log::info!("建立通道只写:{:?}", channel_type);
        self.send(channel.write_to_bytes()?)?;
        Ok(ChannelSender(sender_up, self.id))
    }
    //读写
    pub fn create_channel(
        &mut self,
        channel_type: ChannelType,
    ) -> ResultType<(ChannelSender, ChannelReceiver)> {
        if !self.status.load(std::sync::atomic::Ordering::SeqCst) {
            Err(RemoteError::Disconnection)?;
        }
        let (sender_down, receiver_down) = sync_channel::<Vec<u8>>(10);
        self.id += 2;
        self.channel_map.insert(self.id, sender_down);
        let channel = ChannelMover {
            id: self.id,
            control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Create),
            channel_type: protobuf::ProtobufEnumOrUnknown::new(channel_type),
            channel_power: protobuf::ProtobufEnumOrUnknown::new(ChannelPower::Both),
            ..Default::default()
        };
        let sender_up = self.sender_in.clone();
        self.send(channel.write_to_bytes()?)?;
        return Ok((
            ChannelSender(sender_up, self.id),
            ChannelReceiver(receiver_down),
        ));
    }
    pub fn destroy_channel(&mut self, channel_id: u32) -> ResultType<()> {
        self.channel_map.remove(&channel_id);
        let channel_mover = ChannelMover {
            id: channel_id,
            control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Destroy),
            ..Default::default()
        };
        self.send(channel_mover.write_to_bytes()?)?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ChannelSender(SyncSender<Vec<u8>>, u32);

impl ChannelSender {
    pub fn channel_id(&self) -> u32 {
        self.1
    }
    pub fn send(&mut self, data: Vec<u8>) -> ResultType<()> {
        let channel_mover = ChannelMover {
            id: self.1,
            data,
            control: protobuf::ProtobufEnumOrUnknown::new(ChannelControl::Data),
            ..Default::default()
        };
        self.0.send(channel_mover.write_to_bytes()?)?;
        Ok(())
    }
}
impl Deref for ChannelSender {
    type Target = SyncSender<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChannelSender {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct ChannelReceiver(Receiver<Vec<u8>>);
impl Deref for ChannelReceiver {
    type Target = Receiver<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChannelReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
