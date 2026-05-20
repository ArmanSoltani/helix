use anyhow::Error;
use rand::distr::{Alphanumeric, SampleString};
use tokio::net::UnixListener;

const HELIX_IPC_ENABLE: &str = "HELIX_IPC_ENABLE";

pub struct IPCCommandsStream {
    listener: UnixListener,
}

impl IPCCommandsStream {
    pub fn new() -> Result<Option<Self>, Error> {
        if let Err(err) = std::env::var(HELIX_IPC_ENABLE) {
            log::info!("helix ipc not enable through environment variable: {err}");
            return Ok(None);
        }

        let socket_rand_id = Alphanumeric.sample_string(&mut rand::rng(), 16);
        let socket_name = format!("/tmp/hx-{socket_rand_id}");

        let listener = UnixListener::bind(socket_name)?;

        Ok(Some(Self { listener }))
    }
}
