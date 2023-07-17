use anyhow::Result;
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;

const SOCK_ADDR: &str = "/tmp/humsh.sock";

pub trait Ipc {
    fn send(&mut self, cmd: String) -> Result<()>;
    fn recv(&mut self) -> Result<String>;
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
    fn execute(&self, cmd: String) -> Result<String> {
        // TODO: replace all this with a nice struct which will represent this data
        let mut parts = cmd.split_whitespace();

        if let Some(program) = parts.next() {
            let args: Vec<_> = parts.collect();

            let child = Command::new(program).args(args).spawn();

            match child {
                Ok(child) => {
                    // Wait for the child process to finish
                    let result = child.wait_with_output();
                    match result {
                        Ok(status) => {
                            // the output of the command on STDOUT
                            // TODO: handle STDERR as well
                            return Ok(String::from_utf8(status.stdout)?);
                        }
                        Err(err) => {
                            println!("Failed to wait for child process: {}", err);
                        }
                    }
                }
                Err(err) => {
                    println!("Failed to spawn child process: {}", err);
                }
            }
        } else {
            println!("Invalid command format");
        }
        Ok(String::new())
    }
}

// Send and receive information from other processes through a Unix socket
//
// The socket is already predefined
impl Ipc for Socket {
    fn send(&mut self, cmd: String) -> Result<()> {
        let mut stream = UnixStream::connect(SOCK_ADDR)?;
        stream.write_all(cmd.as_bytes())?;
        Ok(())
    }
    fn recv(&mut self) -> Result<String> {
        match self.socktype {
            SocketType::Listener => {
                let listener = self.listener.as_ref().unwrap();
                for rawconn in listener.incoming() {
                    match rawconn {
                        Ok(mut conn) => {
                            let mut buf = String::new();
                            conn.read_to_string(&mut buf)?;
                            return Ok(buf);
                        }
                        Err(_) => {}
                    }
                }
            }
            SocketType::Stream => match UnixStream::connect(SOCK_ADDR) {
                Ok(mut stream) => {
                    let mut buf = String::new();
                    stream.read_to_string(&mut buf)?;
                    return Ok(buf);
                }
                Err(_) => {}
            },
        }
        Ok(String::new())
    }
}

pub fn listener() {
    let mut conn = Socket::new().unwrap();
    match conn.recv() {
        Ok(cmd) => {
            let output = conn.execute(cmd).unwrap();
            conn.send(output).unwrap();
        }
        Err(_) => {}
    };
    std::fs::remove_file(SOCK_ADDR).unwrap();
}
