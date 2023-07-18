#![allow(dead_code)]
use anyhow::Result;

mod command_line;
mod data;
mod ipc;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::task::spawn(async move {
        loop {
            ipc::listener().await;
        }
    });
    let program = data::git_push();
    ui::Ui::new(program).run().await?;
    Ok(())
}
