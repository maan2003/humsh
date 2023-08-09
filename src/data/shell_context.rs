use crate::config::Config;
use anyhow::{Context, Result};
use dirs;
use once_cell::unsync::OnceCell;
use std::cell::Cell;
use std::path::Path;
use std::process::Command;

pub struct ShellContext {
    user_config: OnceCell<Option<Config>>,
    project_config: OnceCell<Option<Config>>,
    is_git: Cell<Option<bool>>,
}

impl ShellContext {
    pub fn new() -> Self {
        Self {
            user_config: OnceCell::new(),
            project_config: OnceCell::new(),
            is_git: Cell::new(None),
        }
    }

    pub fn is_git(&self) -> bool {
        if let Some(result) = self.is_git.get() {
            return result;
        }

        let result = Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| output.stdout.starts_with(b"true"))
            .unwrap_or(false);
        self.is_git.set(Some(result));
        result
    }

    pub fn user_config(&self) -> Result<Option<&Config>> {
        self.user_config
            .get_or_try_init(|| {
                let path = dirs::config_dir()
                    .context("config dir not found")?
                    .join("humsh/commands.toml");
                if !path.exists() {
                    return Ok(None);
                }
                Config::read(&path)
                    .with_context(|| format!("Failed to read user config from {}", path.display()))
                    .map(Some)
            })
            .map(Option::as_ref)
    }

    pub fn project_config(&self) -> Result<Option<&Config>> {
        self.project_config
            .get_or_try_init(|| {
                let path = Path::new(".humsh/commands.toml");
                if !path.exists() {
                    return Ok(None);
                }
                Config::read(&path)
                    .with_context(|| {
                        format!("Failed to read project config from {}", path.display())
                    })
                    .map(Some)
            })
            .map(Option::as_ref)
    }
}
