use std::{env, io, process::Command};

use anyhow::Context as _;

use crate::util::CheckExitStatus;

enum TermDetect {
    Tmux,
}

pub struct MultiTerm {
    kind: TermDetect,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct TabHandle {
    window_id: String,
    name: String,
    number: u64,
}

impl TabHandle {
    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl MultiTerm {
    pub fn run(&mut self, command: &mut Command) -> anyhow::Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.arg("new-window")
            .arg("-F#{window_id},#{window_name},#{window_index}")
            .arg("-P")
            .envs(
                command
                    .get_envs()
                    .filter_map(|(key, value)| Some((key, value?))),
            );

        for (key, value) in command.get_envs() {
            if let Some(val) = value {
                let key = key.to_str().context("must be valid utf8")?;
                let val = val.to_str().context("must be valid utf8")?;
                cmd.arg(format!("-e{key}={val}"));
            }
        }

        let output = cmd
            .arg(command.get_program())
            .args(command.get_args())
            .output()?
            .check_exit_status()
            .context("running tmux new window")?;

        Ok(csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(io::Cursor::new(output.stdout))
            .deserialize()
            .next()
            .context("invalid tmux output")??)
    }

    pub fn list_windows(&self) -> anyhow::Result<Vec<TabHandle>> {
        let output = Command::new("tmux")
            .arg("list-windows")
            .arg("-F#{window_id},#{window_name},#{window_index}")
            .output()?;

        Ok(csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(io::Cursor::new(output.stdout))
            .deserialize()
            .collect::<Result<_, _>>()?)
    }

    pub fn focus(&self, handle: &TabHandle) -> anyhow::Result<()> {
        let _ = Command::new("tmux")
            .arg("select-window")
            .arg("-t")
            .arg(&handle.window_id)
            .output()?;
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
