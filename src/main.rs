#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod config;
mod data;
mod direnv;
mod multi_term;
mod ui;
mod util;

fn main() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let _guard = rt.enter();
    let program = data::jj()?;
    ui::Ui::new(program)?.run()?;
    Ok(())
}
