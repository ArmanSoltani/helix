use anyhow::Error;
use rand::distr::{Alphanumeric, SampleString};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{UnixListener, UnixStream},
};

pub struct IPCCommandsStream {
    listener: UnixListener,
    stream: Option<BufReader<UnixStream>>,
}

impl IPCCommandsStream {
    pub fn new() -> Result<Self, Error> {
        let socket_rand_id = Alphanumeric.sample_string(&mut rand::rng(), 16);
        let socket_name = format!("/tmp/hx-{socket_rand_id}");

        let listener = UnixListener::bind(socket_name)?;

        Ok(Self {
            listener,
            stream: None,
        })
    }

    pub async fn recv(&mut self) -> Option<String> {
        if let Some(stream) = &mut self.stream {
            let mut line = String::new();
            let num_byte = match stream.read_line(&mut line).await {
                Ok(n) => n,
                Err(err) => {
                    panic!("Fail to read from IPC command socket connection: {err}")
                }
            };
            if num_byte == 0 {
                // socket connection closed by the client
                self.stream = None;
                return None;
            }

            return Some(line);
        }

        let stream = match self.listener.accept().await {
            Ok((s, _)) => s,
            Err(err) => {
                panic!("Fail to accept IPC command socket connection: {err}")
            }
        };
        let mut stream = BufReader::new(stream);

        let mut line = String::new();
        let num_byte = match stream.read_line(&mut line).await {
            Ok(n) => n,
            Err(err) => {
                panic!("Fail to read from IPC command socket connection: {err}")
            }
        };
        if num_byte == 0 {
            // socket connection closed by the client
            self.stream = None;
            return None;
        }

        self.stream = Some(stream);
        Some(line)
    }
}
