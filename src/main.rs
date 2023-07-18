#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod data;
mod ui;

fn main() -> Result<()> {
    let program = data::top();
    ui::Ui::new(program).run()?;
    Ok(())
}
