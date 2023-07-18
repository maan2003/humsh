use anyhow::Result;
use async_trait::async_trait;
use std::io::Read;
use std::process::Command;
use tokio::io::AsyncReadExt;
use tokio::net::{UnixListener, UnixStream};

const SOCK_ADDR: &str = "/tmp/humsh.sock";

#[async_trait]
pub trait Ipc {
    async fn send(&mut self, cmd: String) -> Result<()>;
    async fn recv(&mut self) -> Result<String>;
}

enum SocketType {
    Listener,
    Stream,
}

pub struct Socket {
    socktype: SocketType,
    listener: Option<UnixListener>,
}

impl Socket {
    pub fn new() -> Result<Socket> {
        match UnixListener::bind(SOCK_ADDR) {
            Ok(listener) => Ok(Socket {
                socktype: SocketType::Listener,
                listener: Some(listener),
            }),
            Err(_) => Ok(Socket {
                socktype: SocketType::Stream,
                listener: None,
            }),
        }
    }
    fn execute(&self, cmd: String) -> Result<()> {
        // TODO: replace all this with a nice struct which will represent this data
        let mut parts = cmd.split_whitespace();

        if let Some(program) = parts.next() {
            let args: Vec<_> = parts.collect();
            Command::new(program).args(args).spawn()?;
        }
        Ok(())
    }
}

// Send and receive information from other processes through a Unix socket
//
// The socket is already predefined
#[async_trait]
impl Ipc for Socket {
    async fn send(&mut self, cmd: String) -> Result<()> {
        let stream = UnixStream::connect(SOCK_ADDR).await?;
        stream.try_write(cmd.as_bytes())?;
        Ok(())
    }
    async fn recv(&mut self) -> Result<String> {
        match self.socktype {
            SocketType::Listener => {
                let (stream, _) = self.listener.as_ref().unwrap().accept().await?;
                let mut conn = stream.into_std()?;
                conn.set_nonblocking(false)?;
                let mut buf = String::new();
                conn.read_to_string(&mut buf).unwrap();
                return Ok(buf);
            }
            SocketType::Stream => match UnixStream::connect(SOCK_ADDR).await {
                Ok(mut stream) => {
                    let mut buf = String::new();
                    stream.read_to_string(&mut buf).await?;
                    return Ok(buf);
                }
                Err(_) => {}
            },
        }
        Ok(String::new())
    }
}

pub async fn listener() {
    let mut conn = Socket::new().unwrap();
    match conn.recv().await {
        Ok(cmd) => {
            conn.execute(cmd).unwrap();
        }
        Err(_) => {}
    };
    std::fs::remove_file(SOCK_ADDR).unwrap();
}
