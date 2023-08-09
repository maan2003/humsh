use crate::config::Config;
use anyhow::Context as _;
use dirs;
use once_cell::unsync::OnceCell;
use std::cell::Cell;
use std::path::Path;
use std::process::Command;

pub struct ShellContext {
    user_config: OnceCell<Option<Config>>,
    project_config: OnceCell<Option<Config>>,
    is_git: Cell<Option<bool>>,
    is_cp: Cell<Option<bool>>,
}

impl ShellContext {
    pub fn new() -> Self {
        Self {
            user_config: OnceCell::new(),
            project_config: OnceCell::new(),
            is_git: Cell::new(None),
            is_cp: Cell::new(None),
        }
    }

    pub fn is_git(&self) -> bool {
        if let Some(result) = self.is_git.get() {
            return result;
        }

        let result = self
            .project_config()
            .and_then(|x| x.git)
            .or_else(|| self.user_config().and_then(|x| x.git))
            .unwrap_or_else(detect_git);

        self.is_git.set(Some(result));
        result
    }

    pub fn is_cp(&self) -> bool {
        if let Some(result) = self.is_cp.get() {
            return result;
        }

        let result = self
            .project_config()
            .and_then(|x| x.cp)
            .or_else(|| self.user_config().and_then(|x| x.cp))
            .unwrap_or(false);

        self.is_cp.set(Some(result));
        result
    }

    pub fn user_config(&self) -> Option<&Config> {
        self.user_config
            .get_or_init(|| {
                let path = dirs::config_dir()
                    .context("config dir not found")
                    .ok()?
                    .join("humsh/config.toml");
                if !path.exists() {
                    return None;
                }
                Config::read(&path).ok()
            })
            .as_ref()
    }

    pub fn project_config(&self) -> Option<&Config> {
        self.project_config
            .get_or_init(|| {
                let path = Path::new(".humsh/config.toml");
                if !path.exists() {
                    return None;
                }
                Config::read(&path).ok()
            })
            .as_ref()
    }
}

fn detect_git() -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| output.stdout.starts_with(b"true"))
        .unwrap_or(false)
}
