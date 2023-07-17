#![allow(dead_code)]
use anyhow::Result;
use std::env;

mod command_line;
mod data;
mod ipc;
mod ui;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "listen" {
        ipc::listener();
    } else {
        let program = data::git_push();
        ui::Ui::new(program).run()?;
    }
    Ok(())
}
