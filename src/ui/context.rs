use std::{
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Context as _;

use crate::{command_line::CommandLine, data::Page, direnv::Direnv, util::CheckExitStatus};

use super::{Event, Stdout, Ui};

pub struct Context<'a, 'b> {
    pub(super) stdout: Stdout<'a, 'b>,
    pub(super) ui: &'a mut Ui,
    pub(super) exit: &'a mut bool,
}

#[derive(Debug)]
pub struct ExternalContext {
    tx: flume::Sender<Event>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
pub struct BgTaskId(pub u64);

impl ExternalContext {
    pub(super) fn new(tx: flume::Sender<Event>) -> Self {
        Self { tx }
    }

    pub async fn begin_status(&self, message: impl Into<String>) -> BgTaskId {
        static STATUS_ID: AtomicU64 = AtomicU64::new(0);
        let status_id = BgTaskId(STATUS_ID.fetch_add(0, Ordering::SeqCst));
        self.tx
            .send_async(Event::Task(status_id, message.into()))
            .await
            .expect("ui is not running");
        status_id
    }

    pub async fn update_status(&self, id: BgTaskId, message: impl Into<String>) {
        self.tx
            .send_async(Event::Task(id, message.into()))
            .await
            .expect("ui is not running")
    }

    pub async fn remove_status(&self, id: BgTaskId) {
        self.tx
            .send_async(Event::RemoveStatus(id))
            .await
            .expect("ui is not running")
    }
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

    pub fn external_ctx(&self) -> ExternalContext {
        ExternalContext::new(self.ui.event_tx.clone())
    }

    pub fn push_page(&mut self, page: Page) {
        self.ui.stack.push((self.command_line().clone(), page));
    }

    pub fn currrent_page_mut(&mut self) -> &mut Page {
        self.ui.currrent_page_mut()
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
        self.ui.direnv = Direnv::new(self.external_ctx(), std::env::current_dir()?)?;
        Ok(())
    }

    pub fn run_command_line_other(&mut self, cmd: &CommandLine) -> anyhow::Result<()> {
        self.ui.run_command_line_other(cmd, self.stdout)
    }

    pub fn run_command_line(&mut self) -> anyhow::Result<()> {
        self.ui.run_command_line(self.stdout)
    }

    pub fn run_command_new_term(&mut self, command: &mut process::Command) -> anyhow::Result<()> {
        self.ui.direnv.hook(command)?;
        if let Some(mux) = self.ui.multi_term() {
            mux.run(command)
        } else {
            command.spawn()?.wait()?.check_exit_status()?;
            Ok(())
        }
    }

    pub fn hint_running_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        self.ui.hint_running_command(cmd, self.stdout)?;
        Ok(())
    }

    pub fn run_command_in_foreground(
        &mut self,
        command: &mut process::Command,
    ) -> anyhow::Result<()> {
        self.leave_ui()?;
        self.run_command(command)?.wait()?.check_exit_status()?;
        Ok(())
    }

    pub fn run_command(
        &mut self,
        command: &mut process::Command,
    ) -> anyhow::Result<process::Child> {
        self.ui.direnv.hook(command)?;
        Ok(command.spawn()?)
    }

    pub fn read_input(&mut self, prompt: &str) -> anyhow::Result<String> {
        self.ui.read_input(self.stdout, prompt)
    }
}
