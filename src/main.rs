#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod data;
mod draw;

fn main() -> Result<()> {
    let program = data::git_push();
    let ui = draw::Ui::new(program);
    ui.run()?;
    Ok(())
}
