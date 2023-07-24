use std::{env, process::Command};

enum TermDetect {
    Tmux,
}

pub struct MultiTerm {
    kind: TermDetect,
}

impl MultiTerm {
    pub fn run(&mut self, command: &mut Command) -> anyhow::Result<()> {
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

pub fn detect() -> Option<MultiTerm> {
    if env::var("TMUX").is_ok_and(|x| !x.is_empty()) {
        return Some(MultiTerm {
            kind: TermDetect::Tmux,
        });
    }
    None
}
