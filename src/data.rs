mod shell_context;

use std::borrow::Cow;
use std::process::Command;

use std::{process::Stdio, sync::Arc};

use anyhow::Result;

use crate::command_line::{Arg, ArgOrder, ArgValue, CommandLine};
use crate::ui::Context;

use self::shell_context::ShellContext;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub String);

#[derive(Debug, Clone)]
pub struct Button {
    pub key: Keybind,
    pub description: String,
    pub handler: Arc<dyn ButtonHandler>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub description: String,
    pub buttons: Vec<Button>,
}

#[derive(Clone)]
pub struct Page {
    pub status: Option<Arc<dyn Fn() -> anyhow::Result<String>>>,
    pub status_cache: Option<String>,
    pub groups: Vec<Group>,
}

#[derive(Clone)]
pub struct Program {
    pub base: CommandLine,
    pub start: Page,
}

pub enum ButtonValue<'a> {
    String {
        name: &'a str,
        value: Option<Cow<'a, str>>,
    },
    Bool {
        name: &'a str,
        value: bool,
    },
}

pub trait ButtonHandler {
    fn run(&self, ctx: Context<'_, '_>) -> anyhow::Result<()>;
    fn value<'a>(&'a self, command_line: &'a CommandLine) -> Option<ButtonValue<'a>> {
        let _ = command_line;
        None
    }
    fn as_any(&self) -> &dyn std::any::Any;
}

impl std::fmt::Debug for dyn ButtonHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback").finish_non_exhaustive()
    }
}

impl<F> ButtonHandler for F
where
    F: Fn(Context) -> anyhow::Result<()> + 'static,
{
    fn run(&self, ctx: Context<'_, '_>) -> anyhow::Result<()> {
        self(ctx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as _
    }
}

impl Page {
    pub fn empty() -> Self {
        Self {
            status: None,
            status_cache: None,
            groups: Vec::new(),
        }
    }

    pub fn with_status<F>(mut self, status: F) -> Self
    where
        F: Fn() -> anyhow::Result<String> + 'static,
    {
        self.status = Some(Arc::new(status));
        self
    }

    pub fn refresh_status(&mut self) -> anyhow::Result<()> {
        self.status_cache = self.status.as_ref().map(|x| x()).transpose()?;
        Ok(())
    }

    pub fn status(&self) -> Option<&str> {
        self.status_cache.as_deref()
    }

    pub fn add_group(&mut self, group: Group) {
        self.groups.push(group);
    }
}

fn select_branch(extra_args: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(format!(
            "git branch --list {extra_args} --format '%(refname:short)' | fzf"
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let mut output_text = String::from_utf8(output.stdout)?;
    output_text.truncate(output_text.trim_end().len());
    Ok(output_text)
}

fn select_directory() -> anyhow::Result<String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(r#"cat <(zoxide query -l) <(fd --follow --maxdepth 3 -t d . "$PWD") | sed "s:/$::" | awk '!seen[$0]++' | fzf --tiebreak=end,index"#)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let mut output_text = String::from_utf8(output.stdout)?;
    output_text.truncate(output_text.trim_end().len());
    Ok(output_text)
}

pub struct ToggleFlag(pub Cow<'static, str>);

impl ButtonHandler for ToggleFlag {
    fn run(&self, mut ctx: Context<'_, '_>) -> anyhow::Result<()> {
        ctx.command_line_mut()
            .toggle_arg(Arg::switch(self.0.as_ref()));
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as _
    }

    fn value(&self, command_line: &CommandLine) -> Option<ButtonValue<'_>> {
        Some(ButtonValue::Bool {
            name: self.0.as_ref(),
            value: command_line.args.contains(&Arg::switch(self.0.as_ref())),
        })
    }
}

pub struct PromptButton {
    f: Box<dyn Fn(&mut Context<'_, '_>) -> anyhow::Result<Vec<String>>>,
    arg: String,
}

impl ButtonHandler for PromptButton {
    fn run(&self, mut ctx: Context<'_, '_>) -> anyhow::Result<()> {
        let cmd_line = ctx.command_line();
        if self.get_value(cmd_line).is_some() {
            self.unset_value(ctx.command_line_mut());
        } else {
            let value = (self.f)(&mut ctx)?;
            self.set_values(ctx.command_line_mut(), value);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as _
    }

    fn value<'a>(&'a self, command_line: &'a CommandLine) -> Option<ButtonValue<'a>> {
        Some(ButtonValue::String {
            name: &self.arg,
            value: self.get_value(command_line).map(Cow::Owned),
        })
    }
}

impl PromptButton {
    fn get_value(&self, cmd_line: &CommandLine) -> Option<String> {
        cmd_line.args.iter().find_map(|arg| match &arg.value {
            ArgValue::Multi(m) if m.first()? == &self.arg => Some(
                m.iter()
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 1)
                    .map(|(_, x)| x.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => None,
        })
    }
    fn set_values(&self, cmd_line: &mut CommandLine, values: Vec<String>) {
        let mut args = vec![];
        for value in values {
            args.push(self.arg.clone());
            args.push(value);
        }

        cmd_line.add_arg(Arg::new(ArgOrder::FLAG, ArgValue::Multi(args)));
    }

    fn unset_value(&self, cmd_line: &mut CommandLine) {
        let arg = cmd_line
            .args
            .iter()
            .find_map(|arg| match &arg.value {
                ArgValue::Multi(m) if m.first()? == &self.arg => Some(arg),
                _ => None,
            })
            .cloned();
        if let Some(arg) = arg {
            cmd_line.args.remove(&arg);
        }
    }
}

pub fn page(groups: impl Into<Vec<Group>>) -> Page {
    Page {
        status: None,
        status_cache: None,
        groups: groups.into(),
    }
}

pub fn group(description: impl Into<String>, buttons: impl Into<Vec<Button>>) -> Group {
    Group {
        description: description.into(),
        buttons: buttons.into(),
    }
}

pub fn button(
    key: impl Into<String>,
    description: impl Into<String>,
    handler: impl Fn(Context) -> anyhow::Result<()> + 'static,
) -> Button {
    Button {
        key: Keybind(key.into()),
        description: description.into(),
        handler: Arc::new(handler),
    }
}

pub fn prompt_button(
    key: impl Into<String>,
    description: impl Into<String>,
    name: &str,
    handler: impl Fn(&mut Context) -> anyhow::Result<Vec<String>> + 'static,
) -> Button {
    Button {
        key: Keybind(key.into()),
        description: description.into(),
        handler: Arc::new(PromptButton {
            f: Box::new(handler),
            arg: name.to_string(),
        }),
    }
}

pub fn flag_button(key: &'static str, description: &str, flag: &'static str) -> Button {
    Button {
        key: Keybind(key.into()),
        description: description.into(),
        handler: Arc::new(ToggleFlag(Cow::Borrowed(flag))),
    }
}

pub fn top() -> anyhow::Result<Program> {
    let start = home_page()?;
    Ok(Program {
        base: CommandLine::from_iter([]),
        start,
    })
}

fn home_page() -> Result<Page, anyhow::Error> {
    let shell_context = ShellContext::new();
    let mut builtin_buttons = vec![
        button("c", "Change Directory", |mut ctx| {
            ctx.change_dir(select_directory()?)?;
            ctx.replace_page(home_page()?);
            Ok(())
        }),
        // button("S", "Shell Command", |mut ctx| {
        //     // TODO: run shell commmand from history
        //     let input = ctx.read_input()?;
        //     ctx.leave_ui()?;
        //     ctx.show_cmd()?;
        //     ctx.hint_running_command(&input)?;
        //     let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
        //     ctx.run_command_in_foreground(&mut Command::new(shell).arg("-c").arg(input))?;
        //     Ok(())
        // }),
        button("s", "Shell", |mut ctx| {
            ctx.leave_ui()?;
            let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
            ctx.run_command_in_foreground(&mut Command::new(shell))?;
            Ok(())
        }),
    ];
    if shell_context.is_git() {
        builtin_buttons.push(button("j", "Jujutsu", |mut ctx| {
            ctx.push_page(jj::jj()?);
            ctx.command_line_mut().add_arg(Arg::program("jj"));
            Ok(())
        }));
        builtin_buttons.push(button("e", "Edit", |mut ctx| {
            ctx.leave_ui()?;
            // TODO: handle EDITOR
            ctx.run_command_new_term(Command::new("hx").arg("."))?;
            Ok(())
        }));
    }
    let mut page = page([group("Builtin commands", builtin_buttons)]);
    if let Some(config) = shell_context.user_config() {
        page.add_group(group("User commands", config.command_buttons()));
    }

    if let Some(config) = shell_context.project_config() {
        page.add_group(group("Project commands", config.command_buttons()));
    }
    Ok(page)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageAction {
    Pop,
    None,
}

pub fn exec_cmd(ctx: &mut Context, args: Vec<Arg>) -> anyhow::Result<()> {
    let mut cmd = ctx.command_line().clone();
    for arg in &args {
        cmd.add_arg(arg.clone());
    }
    ctx.run_command_line_other(&cmd)?;
    Ok(())
}

pub fn prompt_arg(
    ctx: &mut Context,
    prompt_fn: impl Fn(&mut Context) -> Result<Vec<Arg>>,
) -> anyhow::Result<()> {
    let args = prompt_fn(ctx)?;
    for arg in args {
        ctx.command_line_mut().add_arg(arg);
    }
    Ok(())
}

pub fn subcommand_button<I>(key: &'static str, description: &str, args: I, page: Page) -> Button
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let arg = Arg::subcommands(args);
    button(key, description, move |mut ctx| {
        ctx.push_page(page.clone());
        ctx.command_line_mut().add_arg(arg.clone());
        Ok(())
    })
}

pub fn exec_button(
    key: &'static str,
    description: &str,
    args: impl IntoIterator<Item = Arg>,
    page_action: PageAction,
) -> Button {
    let args: Vec<_> = args.into_iter().collect();
    button(key, description, move |mut ctx| {
        let result = exec_cmd(&mut ctx, args.clone());
        match page_action {
            PageAction::Pop => {
                ctx.pop_page();
            }
            PageAction::None => {}
        }
        result
    })
}

pub fn exec_button_arg_prompt(
    key: &'static str,
    description: &str,
    args: impl IntoIterator<Item = Arg>,
    page_action: PageAction,
    prompt_fn: impl Fn(&mut Context) -> Result<Vec<Arg>> + 'static,
) -> Button {
    let args: Vec<_> = args.into_iter().collect();
    button(key, description, move |mut ctx| {
        let result = prompt_arg(&mut ctx, &prompt_fn);
        let result = result.and_then(|_| exec_cmd(&mut ctx, args.clone()));
        match page_action {
            PageAction::Pop => {
                ctx.pop_page();
            }
            PageAction::None => {}
        }
        result
    })
}

pub fn args_page(args: impl Into<Vec<Button>>, actions: impl Into<Vec<Button>>) -> Page {
    page([
        group("Arguments", args.into()),
        group("Action", actions.into()),
    ])
}

pub fn subcommand_page_button<I>(
    key: &'static str,
    name: &str,
    cmd_args: I,
    args: impl Into<Vec<Button>>,
    actions: impl Into<Vec<Button>>,
) -> Button
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    subcommand_button(key, name, cmd_args, args_page(args, actions))
}

mod jj;
