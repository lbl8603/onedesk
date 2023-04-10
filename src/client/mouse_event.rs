use crossbeam::atomic::AtomicCell;
use stream::{remote_channel::ChannelSender, ResultType};

lazy_static::lazy_static! {
    static ref SENDER_CELL:AtomicCell<Option<ChannelSender>> =  AtomicCell::new(None);
}

pub fn load(sender: ChannelSender) {
    SENDER_CELL.store(Some(sender));
}
pub fn send(data: Vec<u8>) -> ResultType<()> {
    if let Some(mut sender) = SENDER_CELL.take() {
        sender.send(data)?;
        SENDER_CELL.store(Some(sender));
    }
    Ok(())
}
