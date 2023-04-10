use stream::tokio::runtime::{Builder, Runtime};

lazy_static::lazy_static! {
    pub static ref TOKIO_RUNTIME: Runtime = Builder::new_multi_thread()
    .worker_threads(4)
    .enable_all()
    .build()
    .unwrap();
}
