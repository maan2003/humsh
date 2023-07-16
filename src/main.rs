use anyhow::Result;

mod command_line;
mod ui;

struct Ui {
    command_line: CommandLine,
    stack: Vec<Group>,
}

fn main() {}
