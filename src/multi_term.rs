use std::{env, io, process::Command};

use anyhow::anyhow;

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
        let output = Command::new("tmux")
            .arg("new-window")
            .arg("-F#{window_id},#{window_name},#{window_index}")
            .arg("-P")
            .envs(
                command
                    .get_envs()
                    .filter_map(|(key, value)| Some((key, value?))),
            )
            .arg(command.get_program())
            .args(command.get_args())
            .output()?;

        Ok(csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(io::Cursor::new(output.stdout))
            .deserialize()
            .next()
            .ok_or_else(|| anyhow!("Invalid tmux output"))??)
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
