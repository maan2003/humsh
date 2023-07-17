use std::ops::ControlFlow;

use crossterm::event::{self, Event};
use crossterm::{cursor, execute, queue, style::*, terminal};

use crate::data::Action;
use crate::{
    command_line::CommandLine,
    data::{self, Button, Group, Page},
};
use input::KeyHandler;

mod input;

pub struct Ui {
    stack: Vec<(CommandLine, Page)>,
    key_handler: KeyHandler,
    showing_cmd: bool,
}

impl Ui {
    pub fn new(program: data::Program) -> Self {
        Self {
            stack: vec![(program.base, program.start)],
            key_handler: KeyHandler::new(),
            showing_cmd: false,
        }
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

    pub fn run(mut self) -> anyhow::Result<()> {
        let mut stdout = std::io::stdout().lock();
        self.enter_ui(&mut stdout)?;
        loop {
            if !self.showing_cmd {
                self.draw(&mut stdout)?;
            }
            if let Some(action) = self.process_event(event::read()?)? {
                if self.handle_action(action, &mut stdout)?.is_break() {
                    break;
                }
            }
        }
        self.leave_ui(&mut stdout)?;
        Ok(())
    }

    pub fn handle_action(
        &mut self,
        action: Action,
        stdout: &mut impl std::io::Write,
    ) -> anyhow::Result<ControlFlow<()>> {
        match action {
            Action::Batch(actions) => {
                for action in actions {
                    if self.handle_action(action, stdout)?.is_break() {
                        return Ok(ControlFlow::Break(()));
                    }
                }
            }
            Action::Toggle(arg) => {
                self.command_line_mut().toggle_arg(arg);
            }
            Action::Add(arg) => {
                self.command_line_mut().add_arg(arg);
            }
            Action::Popup(page) => self.stack.push((self.command_line().clone(), page)),
            Action::Run { exit } => {
                self.leave_ui(stdout)?;
                let cli = self
                    .command_line()
                    .args
                    .iter()
                    .map(|x| x.value.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                execute!(
                    stdout,
                    PrintStyledContent(format!("> {cli}\n").with(Color::DarkGreen))
                )?;
                self.command_line().to_std().spawn()?.wait()?;
                if exit {
                    return Ok(ControlFlow::Break(()));
                }
                self.enter_ui(stdout)?;
            }
            Action::ToggleCmd => {
                if self.showing_cmd {
                    execute!(stdout, terminal::EnterAlternateScreen)?;
                } else {
                    execute!(stdout, terminal::LeaveAlternateScreen)?;
                }
                self.showing_cmd = !self.showing_cmd;
            }
            Action::RunHidingUi(cb) => {
                self.leave_ui(stdout)?;
                let ret = self.handle_action(cb.0()?, stdout)?;
                self.enter_ui(stdout)?;
                return Ok(ret);
            }
            Action::Escape if self.stack.len() == 1 => {
                return Ok(ControlFlow::Break(()));
            }
            Action::Escape => {
                self.stack.pop();
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn enter_ui(&self, stdout: &mut impl std::io::Write) -> crossterm::Result<()> {
        terminal::enable_raw_mode()?;
        execute!(stdout, terminal::EnterAlternateScreen)?;
        Ok(())
    }

    fn leave_ui(&self, stdout: &mut impl std::io::Write) -> crossterm::Result<()> {
        execute!(stdout, terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn process_event(&mut self, event: Event) -> crossterm::Result<Option<Action>> {
        match event {
            Event::Key(key) => {
                let page = &self.stack.last().expect("stack must not be empty").1;
                self.key_handler.handle_key(
                    key,
                    page.groups
                        .iter()
                        .flat_map(|x| &x.buttons)
                        .map(|b| (b.key.clone(), b.action.clone())),
                )
            }
            _ => Ok(None),
        }
    }

    pub fn draw(&self, mut stdout: impl std::io::Write) -> anyhow::Result<()> {
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
        self.draw_page(self.currrent_page(), &mut stdout)?;
        let (_, height) = terminal::size()?;
        queue!(stdout, cursor::MoveTo(0, height - 2))?;
        stdout.flush()?;

        self.draw_prompt(&mut stdout)?;
        Ok(())
    }

    fn draw_prompt(&self, stdout: &mut impl std::io::Write) -> Result<(), anyhow::Error> {
        std::process::Command::new("starship")
            .arg("prompt")
            .spawn()?
            .wait()?;
        execute!(
            stdout,
            Print(self.command_line().to_string()),
            Print(" "),
            Print(self.key_handler.prefix()),
        )?;
        Ok(())
    }

    fn draw_page(
        &self,
        page: &Page,
        stdout: &mut impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        for group in &page.groups {
            self.draw_group(group, stdout)?;
            queue!(stdout, NextLine)?;
        }
        Ok(())
    }

    fn draw_group(&self, group: &Group, stdout: &mut impl std::io::Write) -> crossterm::Result<()> {
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

    fn draw_button(
        &self,
        button: &Button,
        stdout: &mut impl std::io::Write,
    ) -> crossterm::Result<()> {
        queue!(
            stdout,
            Print(" "),
            PrintStyledContent((&*button.key.0).with(Color::Grey)),
            Print(" "),
            Print(&button.description),
        )?;
        if let data::Action::Toggle(a) = &button.action {
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

struct NextLine;
impl crossterm::Command for NextLine {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        Print('\n').write_ansi(f)?;
        cursor::MoveToColumn(0).write_ansi(f)?;
        Ok(())
    }
}
