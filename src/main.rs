#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod data;
mod ipc;
mod ui;

fn main() -> Result<()> {
    let program = data::git_push();
    ui::Ui::new(program).run()?;
    Ok(())
}
