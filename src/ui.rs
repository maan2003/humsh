use std::ffi::OsStr;
use std::io::{StdoutLock, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context as _};
use crossterm::event::{self, Event};
use crossterm::{cursor, execute, queue, style::*, terminal};

use crate::command_line::CommandLine;
use crate::data::{self, Button, Callback, Group, Page, ToggleFlag};
use crate::direnv::Direnv;
use crate::multi_term::{self, MultiTerm};
pub use context::Context;
use input::KeyHandler;

mod context;
mod input;

pub type Stdout<'a, 'b> = &'a mut StdoutLock<'b>;

pub struct Ui {
    stack: Vec<(CommandLine, Page)>,
    key_handler: KeyHandler,
    direnv: Direnv,
    showing_cmd: bool,
    multi_term: Option<Box<dyn MultiTerm + 'static>>,
}

impl Ui {
    pub fn new(program: data::Program) -> anyhow::Result<Self> {
        Ok(Self {
            stack: vec![(program.base, program.start)],
            key_handler: KeyHandler::new(),
            direnv: Direnv::new(std::env::current_dir()?)?,
            showing_cmd: false,
            multi_term: multi_term::detect(),
        })
    }

    pub fn command_line(&self) -> &CommandLine {
        &self.stack.last().expect("stack must not be empty").0
    }

    pub fn command_line_mut(&mut self) -> &mut CommandLine {
        &mut self.stack.last_mut().expect("stack must not be empty").0
    }

    pub fn currrent_page(&self) -> &Page {
        &self.stack.last().expect("stack must not be empty").1
    }

    pub fn currrent_page_mut(&mut self) -> &mut Page {
        &mut self.stack.last_mut().expect("stack must not be empty").1
    }

    pub fn multi_term(&mut self) -> Option<&mut Box<dyn MultiTerm>> {
        self.multi_term.as_mut()
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let mut stdout = std::io::stdout().lock();
        loop {
            terminal::enable_raw_mode()?;
            if !self.showing_cmd {
                self.draw(&mut stdout)?;
            }
            if let Some(callback) = self.process_event(event::read()?)? {
                let mut exit = false;
                let ctx = Context {
                    stdout: &mut stdout,
                    ui: &mut self,
                    exit: &mut exit,
                };
                // FIXME proper error handling
                if let Err(e) = callback.call(ctx) {
                    self.leave_ui(&mut stdout)?;
                    self.showing_cmd = true;
                    execute!(
                        stdout,
                        PrintStyledContent(format!("! {:#}", e).with(Color::Red)),
                        NextLine,
                    )?;
                }
                if exit {
                    break;
                }
            }
        }
        self.leave_ui(&mut stdout)?;
        Ok(())
    }

    fn run_command_line(&mut self, stdout: Stdout) -> anyhow::Result<()> {
        self.leave_ui(stdout)?;
        let cli = self
            .command_line()
            .args
            .iter()
            .map(|x| x.value.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        self.hint_running_command(&cli, stdout)?;
        let mut cmd = self.command_line().to_std();
        self.direnv.hook(&mut cmd)?;
        let status = cmd.spawn()?.wait()?;
        if !status.success() {
            bail!("exit code {}", status.code().unwrap_or(-1));
        }
        Ok(())
    }

    fn hint_running_command(&self, cmd: &str, stdout: Stdout) -> crossterm::Result<()> {
        execute!(
            stdout,
            PrintStyledContent(format!("> {cmd}\n").with(Color::DarkGreen))
        )?;
        Ok(())
    }

    fn enter_ui(&self, stdout: Stdout) -> crossterm::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout, terminal::EnterAlternateScreen)?;
        Ok(())
    }

    fn leave_ui(&self, stdout: Stdout) -> crossterm::Result<()> {
        // always write at end of terminal
        let (_, height) = terminal::size()?;
        execute!(
            stdout,
            terminal::LeaveAlternateScreen,
            cursor::MoveTo(0, height - 1),
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    fn hide_cmd(&mut self, stdout: Stdout) -> crossterm::Result<()> {
        execute!(stdout, terminal::EnterAlternateScreen)?;
        self.showing_cmd = false;
        Ok(())
    }

    fn show_cmd(&mut self, stdout: Stdout) -> crossterm::Result<()> {
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        self.showing_cmd = true;
        Ok(())
    }

    fn toggle_cmd(&mut self, stdout: Stdout) -> crossterm::Result<()> {
        if self.showing_cmd {
            self.hide_cmd(stdout)?;
        } else {
            self.show_cmd(stdout)?;
        }
        Ok(())
    }

    pub fn process_event(&mut self, event: Event) -> crossterm::Result<Option<Arc<dyn Callback>>> {
        match event {
            Event::Key(key) => {
                let page = &self.stack.last().expect("stack must not be empty").1;
                self.key_handler.handle_key(
                    key,
                    page.groups
                        .iter()
                        .flat_map(|x| &x.buttons)
                        .map(|b| (&b.key, &b.callback)),
                )
            }
            _ => Ok(None),
        }
    }

    pub fn draw(&self, stdout: Stdout) -> anyhow::Result<()> {
        self.enter_ui(stdout)?;
        let (_, height) = terminal::size()?;
        // hack: to make terminal keep scrolling
        queue!(
            stdout,
            cursor::MoveTo(0, height - 1),
            terminal::Clear(terminal::ClearType::All)
        )?;
        self.draw_page(self.currrent_page(), stdout)?;
        queue!(stdout, cursor::MoveTo(0, height - 2))?;

        self.draw_prompt(stdout)?;
        stdout.flush()?;
        Ok(())
    }

    fn draw_prompt(&self, stdout: Stdout) -> Result<(), anyhow::Error> {
        let dir = pwd()?;

        let dir_name = dir.file_name().and_then(OsStr::to_str).unwrap_or("/");

        let cmd = self.command_line().to_string();
        queue!(
            stdout,
            NextLine,
            PrintStyledContent(dir_name.with(Color::Cyan)),
            PrintStyledContent(" λ ".with(Color::Yellow)),
            Print(&cmd),
            Print(if cmd.is_empty() { "" } else { " " }),
            Print(self.key_handler.prefix()),
        )?;
        Ok(())
    }

    fn draw_page(&self, page: &Page, stdout: Stdout) -> Result<(), std::io::Error> {
        if let Some(status) = &page.status {
            execute!(stdout, Print(status.replace('\n', "\r\n")), NextLine)?;
        }
        for group in &page.groups {
            self.draw_group(group, stdout)?;
            queue!(stdout, NextLine)?;
        }
        Ok(())
    }

    fn draw_group(&self, group: &Group, stdout: Stdout) -> crossterm::Result<()> {
        queue!(
            stdout,
            PrintStyledContent((&*group.description).with(Color::Blue)),
            NextLine,
        )?;
        for button in &group.buttons {
            self.draw_button(button, stdout)?;
            queue!(stdout, NextLine)?;
        }
        Ok(())
    }

    fn draw_button(&self, button: &Button, stdout: Stdout) -> crossterm::Result<()> {
        queue!(
            stdout,
            Print(" "),
            PrintStyledContent((&*button.key.0).with(Color::Grey)),
            Print(" "),
            Print(&button.description),
        )?;
        if let Some(ToggleFlag(a)) = &button.callback.as_any().downcast_ref::<ToggleFlag>() {
            let selected = self.command_line().args.contains(a);
            queue!(
                stdout,
                Print(" ("),
                PrintStyledContent(a.value.to_string().with(if selected {
                    Color::Cyan
                } else {
                    Color::DarkGrey
                })),
                Print(")")
            )?;
        }
        Ok(())
    }
}

fn pwd() -> Result<PathBuf, anyhow::Error> {
    std::env::var("PWD")
        .map_or_else(|_| std::env::current_dir(), |x| Ok(PathBuf::from(x)))
        .context("getting cwd")
}

struct NextLine;
impl crossterm::Command for NextLine {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        Print("\r\n").write_ansi(f)?;
        cursor::MoveToColumn(0).write_ansi(f)?;
        Ok(())
    }
}
