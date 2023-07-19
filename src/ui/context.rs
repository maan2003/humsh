use std::{path::Path, process};

use crossterm::{execute, terminal};

use crate::{command_line::CommandLine, data::Page};

use super::{Stdout, Ui};

pub struct Context<'a, 'b> {
    pub(super) stdout: Stdout<'a, 'b>,
    pub(super) ui: &'a mut Ui,
    pub(super) exit: &'a mut bool,
}

impl<'a, 'b> Context<'a, 'b> {
    pub fn enter_ui(&mut self) -> anyhow::Result<()> {
        Ok(self.ui.enter_ui(self.stdout)?)
    }

    pub fn leave_ui(&mut self) -> anyhow::Result<()> {
        Ok(self.ui.leave_ui(self.stdout)?)
    }

    pub fn command_line(&self) -> &CommandLine {
        self.ui.command_line()
    }

    pub fn command_line_mut(&mut self) -> &mut CommandLine {
        self.ui.command_line_mut()
    }

    pub fn push_page(&mut self, page: Page) {
        self.ui.stack.push((self.command_line().clone(), page));
    }

    /// Returns whether page was poped.
    pub fn pop_page(&mut self) -> bool {
        if self.ui.stack.len() > 1 {
            self.ui.stack.pop().is_some()
        } else {
            false
        }
    }

    pub fn toggle_cmd(&mut self) -> anyhow::Result<()> {
        if self.ui.showing_cmd {
            execute!(self.stdout, terminal::EnterAlternateScreen)?;
        } else {
            execute!(self.stdout, terminal::LeaveAlternateScreen)?;
        }
        self.ui.showing_cmd = !self.ui.showing_cmd;
        Ok(())
    }

    pub fn exit(&mut self) {
        *self.exit = true;
    }

    pub fn change_dir(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        std::env::set_current_dir(&path)?;
        std::env::set_var("PWD", path.as_ref());
        Ok(())
    }

    pub fn run_command_line(&mut self) -> anyhow::Result<()> {
        self.ui.run_command_line(self.stdout)
    }

    pub fn run_command(
        &mut self,
        command: &mut process::Command,
    ) -> anyhow::Result<process::Child> {
        Ok(command.spawn()?)
    }
}
