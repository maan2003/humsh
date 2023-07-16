#![allow(unused_variables)]

use crossterm::event::{self, Event, KeyCode};
use crossterm::{cursor, execute, queue, style::*, terminal};

use crate::data::Action;
use crate::{
    command_line::CommandLine,
    data::{self, Button, Group, Page},
};

pub struct Ui {
    command_line: CommandLine,
    stack: Vec<Page>,
    current_keys: String,
}

impl Ui {
    pub fn new(program: data::Program) -> Self {
        Self {
            command_line: program.base,
            stack: vec![program.start],
            current_keys: String::new(),
        }
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let mut stdout = std::io::stdout().lock();
        self.enter_ui(&mut stdout)?;
        loop {
            self.draw(&mut stdout)?;
            match self.process_event(event::read()?)? {
                Some(Action::Toggle(arg)) => {
                    self.command_line.toggle_arg(arg);
                    self.leave_ui(&mut stdout)?;
                    dbg!(&self.command_line);
                    self.enter_ui(&mut stdout)?;
                }
                Some(Action::Popup(_)) => todo!(),
                Some(Action::Run) => todo!(),
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
            Event::Key(k) => match k.code {
                KeyCode::Char(c) => return self.process_key(c),
                KeyCode::Esc | KeyCode::F(9) => {
                    return Ok(Some(Action::Escape));
                }
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    pub fn process_key(&mut self, key: char) -> crossterm::Result<Option<Action>> {
        self.current_keys.push(key);
        let page = self.stack.last().expect("stack must not be empty");
        for b in page.groups.iter().flat_map(|x| x.buttons.iter()) {
            if b.key.0 == self.current_keys {
                let action = b.action.clone();
                drop(page);
                self.current_keys = String::new();
                return Ok(Some(action));
            } else if b.key.0.starts_with(&self.current_keys) {
                return Ok(None);
            }
        }
        self.current_keys = String::new();
        Ok(None)
    }

    pub fn draw(&self, mut stdout: impl std::io::Write) -> crossterm::Result<()> {
        let page = self.stack.last().expect("stack must not be empty");
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::All)
        )?;
        self.draw_page(page, &mut stdout)?;
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
            PrintStyledContent((&*group.description).with(Color::Cyan)),
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
            queue!(
                stdout,
                Print(" ("),
                PrintStyledContent((&*a.value).with(Color::DarkGrey)),
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
