use anyhow::Result;
use crossterm::{execute, style::*};
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;

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
    fn execute(&self, cmd: String) -> Result<String> {
        let mut stdout = std::io::stdout().lock();
        execute!(
            stdout,
            PrintStyledContent(format!("> {cmd}\n").with(Color::DarkGreen))
        )?;
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
        for rawconn in self.listener.incoming() {
            std::fs::remove_file(SOCK_ADDR)?;
            match rawconn {
                Ok(mut conn) => {
                    let mut buf = String::new();
                    conn.read_to_string(&mut buf).unwrap();
                    return self.execute(buf);
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
