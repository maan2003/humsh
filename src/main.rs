#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod config;
mod data;
mod direnv;
mod multi_term;
mod ui;

fn main() -> Result<()> {
    let program = data::top()?;
    ui::Ui::new(program)?.run()?;
    Ok(())
}
