#![allow(unused_imports, unused_mut, unused_variables)]

use crossterm::{
    cursor::{Hide, MoveToColumn, MoveToNextLine, Show},
    queue,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use crate::{
    command_line::CommandLine,
    data::{self, Button, Group, Page},
};

pub struct Ui {
    command_line: CommandLine,
    stack: Vec<Page>,
}

impl Ui {
    pub fn new(program: data::Program) -> Self {
        Self {
            command_line: program.base,
            stack: vec![program.start],
        }
    }

    pub fn draw(&self, mut stdout: impl std::io::Write) -> crossterm::Result<()> {
        let page = self.stack.last().expect("stack must not be empty");
        self.draw_page(page, &mut stdout)?;
        Ok(())
    }

    pub fn undraw(&self, mut stdout: impl std::io::Write) -> crossterm::Result<()> {
        Ok(())
    }

    fn draw_page(
        &self,
        page: &Page,
        stdout: &mut impl std::io::Write,
    ) -> Result<(), std::io::Error> {
        queue!(stdout, NextLine)?;
        for group in &page.groups {
            self.draw_group(group, stdout)?;
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
        queue!(stdout, NextLine)?;
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
            Print(&button.key.0),
            Print(" "),
            Print(&button.description),
        )?;
        Ok(())
    }
}

struct NextLine;
impl crossterm::Command for NextLine {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        Print('\n').write_ansi(f)?;
        MoveToColumn(0).write_ansi(f)?;
        Ok(())
    }
}
