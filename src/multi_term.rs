use std::{env, process::Command};

pub trait MultiTerm {
    /// Command::envs and Command::args must be respected.
    // TODO: return a handle
    fn run(&mut self, command: &mut Command) -> anyhow::Result<()>;
}

struct Tmux;

impl MultiTerm for Tmux {
    fn run(&mut self, command: &mut Command) -> anyhow::Result<()> {
        Command::new("tmux")
            .arg("new-window")
            .envs(
                command
                    .get_envs()
                    .filter_map(|(key, value)| Some((key, value?))),
            )
            .arg(command.get_program())
            .args(command.get_args())
            .spawn()?
            .wait()?;
        Ok(())
    }
}

pub fn detect() -> Option<Box<dyn MultiTerm>> {
    if env::var("TMUX").is_ok_and(|x| !x.is_empty()) {
        return Some(Box::new(Tmux));
    }
    None
}
