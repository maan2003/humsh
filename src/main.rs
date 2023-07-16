#![allow(dead_code)]
use std::{io::Write, time::Duration};

use anyhow::Result;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

mod command_line;
mod data;
mod draw;

fn main() -> Result<()> {
    let program = data::git_push();
    let ui = draw::Ui::new(program);
    let mut stdout = std::io::stdout().lock();
    enable_raw_mode()?;
    ui.draw(&mut stdout)?;
    stdout.flush()?;
    std::thread::sleep(Duration::from_secs(5));
    ui.undraw(&mut stdout)?;
    disable_raw_mode()?;
    Ok(())
}
