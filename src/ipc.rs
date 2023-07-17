use anyhow::Result;
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};

const SOCK_ADDR: &str = "/tmp/humsh.sock";

pub trait Ipc {
    fn send(&mut self, cmd: String) -> Result<()>;
    fn recv(&mut self) -> Result<String>;
}

pub struct Socket {
    listener: UnixListener,
}

impl Socket {
    fn new() -> Result<Socket> {
        Ok(Socket {
            listener: UnixListener::bind(SOCK_ADDR)?,
        })
    }
}

impl Ipc for Socket {
    fn send(&mut self, cmd: String) -> Result<()> {
        let mut stream = UnixStream::connect(SOCK_ADDR)?;
        stream.write_all(cmd.as_bytes())?;
        Ok(())
    }
    fn recv(&mut self) -> Result<String> {
        for rawconn in self.listener.incoming() {
            std::fs::remove_file(SOCK_ADDR)?;
            match rawconn {
                Ok(mut conn) => {
                    let mut buf = String::new();
                    conn.read_to_string(&mut buf).unwrap();
                    return Ok(buf);
                }
                Err(_) => {
                    break;
                }
            }
        }
        Ok(String::new())
    }
}

pub fn test() {
    let mut conn = Socket::new().unwrap();
    let cmd = String::from("git push");
    conn.send(cmd).unwrap();
    match conn.recv() {
        Ok(cmd) => {
            println!("{}", cmd);
        }
        Err(_) => {}
    };
}
