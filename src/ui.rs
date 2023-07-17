#![allow(unused_variables)]

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
    command_line: CommandLine,
    stack: Vec<Page>,
    key_handler: KeyHandler,
    showing_cmd: bool,
}

impl Ui {
    pub fn new(program: data::Program) -> Self {
        Self {
            command_line: program.base,
            stack: vec![program.start],
            key_handler: KeyHandler::new(),
            showing_cmd: false,
        }
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let mut stdout = std::io::stdout().lock();
        self.enter_ui(&mut stdout)?;
        loop {
            if !self.showing_cmd {
                self.draw(&mut stdout)?;
            }
            match self.process_event(event::read()?)? {
                Some(Action::Toggle(arg)) => {
                    self.command_line.toggle_arg(arg);
                }
                Some(Action::Popup(page)) => self.stack.push(page),
                Some(Action::Run { exit }) => {
                    self.leave_ui(&mut stdout)?;
                    let cli = self
                        .command_line
                        .args
                        .iter()
                        .map(|x| x.value.to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    execute!(
                        stdout,
                        PrintStyledContent(format!("> {cli}\n").with(Color::DarkGreen))
                    )?;
                    self.command_line.to_std().spawn()?.wait()?;
                    if exit {
                        break Ok(());
                    }
                    self.enter_ui(&mut stdout)?;
                }
                Some(Action::ToggleCmd) => {
                    if self.showing_cmd {
                        execute!(stdout, terminal::EnterAlternateScreen)?;
                    } else {
                        execute!(stdout, terminal::LeaveAlternateScreen)?;
                    }
                    self.showing_cmd = !self.showing_cmd;
                }
                Some(Action::Escape) if self.stack.len() == 1 => {
                    self.leave_ui(&mut stdout)?;
                    break Ok(());
                }
                Some(Action::Escape) => {
                    self.stack.pop();
                }
                None => {}
            }
        }
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
                let page = self.stack.last().expect("stack must not be empty");
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

    pub fn draw(&self, mut stdout: impl std::io::Write) -> crossterm::Result<()> {
        let page = self.stack.last().expect("stack must not be empty");
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
        self.draw_page(page, &mut stdout)?;
        queue!(stdout, Print(self.key_handler.prefix()))?;
        stdout.flush()?;
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
            let selected = self.command_line.args.contains(a);
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
