use std::{path::Path, process};

use anyhow::Context as _;

use crate::{command_line::CommandLine, data::Page, direnv::Direnv};

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

    pub fn replace_page(&mut self, page: Page) {
        *self.ui.currrent_page_mut() = page;
    }

    /// Returns whether page was poped.
    pub fn pop_page(&mut self) -> bool {
        if self.ui.stack.len() > 1 {
            self.ui.stack.pop().is_some()
        } else {
            false
        }
    }

    pub fn showing_cmd(&self) -> bool {
        self.ui.showing_cmd
    }

    pub fn show_cmd(&mut self) -> anyhow::Result<()> {
        Ok(self.ui.show_cmd(self.stdout)?)
    }

    pub fn hide_cmd(&mut self) -> anyhow::Result<()> {
        Ok(self.ui.hide_cmd(self.stdout)?)
    }

    pub fn toggle_cmd(&mut self) -> anyhow::Result<()> {
        Ok(self.ui.toggle_cmd(self.stdout)?)
    }

    pub fn exit(&mut self) {
        *self.exit = true;
    }

    pub fn change_dir(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        std::env::set_current_dir(&path).context("cd failed")?;
        std::env::set_var("PWD", path.as_ref());
        self.ui.direnv = Direnv::new(std::env::current_dir()?)?;
        Ok(())
    }

    pub fn run_command_line(&mut self) -> anyhow::Result<()> {
        self.ui.run_command_line(self.stdout)
    }

    pub fn run_command_new_term(&mut self, command: &mut process::Command) -> anyhow::Result<()> {
        self.ui.direnv.hook(command)?;
        if let Some(mux) = self.ui.multi_term() {
            mux.run(command)
        } else {
            command.spawn()?.wait()?;
            Ok(())
        }
    }

    pub fn run_command(
        &mut self,
        command: &mut process::Command,
    ) -> anyhow::Result<process::Child> {
        self.ui.direnv.hook(command)?;
        Ok(command.spawn()?)
    }
}
