use std::io::Write;

use crossterm::event::{self, Event, KeyCode};
use crossterm::{cursor, queue, style::*, terminal};

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

    pub fn run(&self) -> anyhow::Result<()> {
        let mut stdout = std::io::stdout().lock();
        terminal::enable_raw_mode()?;
        queue!(stdout, terminal::EnterAlternateScreen)?;
        loop {
            self.draw(&mut stdout)?;
            if !self.process_event(event::read()?)? {
                break;
            }
        }
        queue!(stdout, terminal::LeaveAlternateScreen)?;
        stdout.flush()?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    pub fn process_event(&self, event: Event) -> crossterm::Result<bool> {
        match event {
            Event::Key(k) => match k.code {
                KeyCode::Char(c) => self.process_key(c)?,
                KeyCode::Esc | KeyCode::F(9) => {
                    return Ok(false);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(true)
    }

    pub fn process_key(&self, key: char) -> crossterm::Result<()> {
        Ok(())
    }

    pub fn draw(&self, mut stdout: impl std::io::Write) -> crossterm::Result<()> {
        let page = self.stack.last().expect("stack must not be empty");
        queue!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(terminal::ClearType::Purge)
        )?;
        self.draw_page(page, &mut stdout)?;
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
