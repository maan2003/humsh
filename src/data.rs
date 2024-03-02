mod shell_context;

use std::process::Command;
use std::sync::Mutex;
use std::{process::Stdio, sync::Arc};

use anyhow::Context as _;

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
    pub hint: Option<String>,
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

pub trait ButtonHandler {
    fn run(&self, ctx: Context<'_, '_>) -> anyhow::Result<()>;
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

impl Button {
    pub fn with_hint(mut self, hint: impl Into<Option<String>>) -> Self {
        self.hint = hint.into();
        self
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

pub struct ToggleFlag(pub Arg);

impl ButtonHandler for ToggleFlag {
    fn run(&self, mut ctx: Context<'_, '_>) -> anyhow::Result<()> {
        ctx.command_line_mut().toggle_arg(self.0.clone());
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as _
    }
}
fn toggle_flag(flag: &str) -> ToggleFlag {
    ToggleFlag(Arg::switch(flag))
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
    handler: impl ButtonHandler + 'static,
) -> Button {
    Button {
        key: Keybind(key.into()),
        description: description.into(),
        handler: Arc::new(handler),
        hint: None,
    }
}

pub fn flag_button(key: &'static str, description: &str, flag: &str) -> Button {
    button(key, description, ToggleFlag(Arg::switch(flag)))
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
        button("c", "Change Directory", |mut ctx: Context| {
            ctx.change_dir(select_directory()?)?;
            ctx.replace_page(home_page()?);
            Ok(())
        }),
        // button("S", "Shell Command", |mut ctx: Context| {
        //     // TODO: run shell commmand from history
        //     let input = ctx.read_input()?;
        //     ctx.leave_ui()?;
        //     ctx.show_cmd()?;
        //     ctx.hint_running_command(&input)?;
        //     let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
        //     ctx.run_command_in_foreground(&mut Command::new(shell).arg("-c").arg(input))?;
        //     Ok(())
        // }),
        button("s", "Shell", |mut ctx: Context| {
            ctx.leave_ui()?;
            let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
            ctx.run_command_in_foreground(&mut Command::new(shell))?;
            Ok(())
        }),
    ];
    if shell_context.is_git() {
        builtin_buttons.push(button("j", "Jujutsu", |mut ctx: Context| {
            ctx.push_page(jj::jj()?);
            ctx.command_line_mut().add_arg(Arg::program("jj"));
            Ok(())
        }));
        builtin_buttons.push(button("e", "Edit", |mut ctx: Context| {
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

pub fn prompt_arg(ctx: &mut Context, arg_name: &str) -> anyhow::Result<()> {
    let input = ctx.read_input(arg_name)?;
    ctx.command_line_mut()
        .add_arg(Arg::switch(format!("--{arg_name}={input}")));
    Ok(())
}

pub fn subcommand_button<I>(key: &'static str, description: &str, args: I, page: Page) -> Button
where
    I: IntoIterator,
    I::Item: Into<String>,
{
    let arg = Arg::subcommands(args);
    button(key, description, move |mut ctx: Context| {
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
    button(key, description, move |mut ctx: Context| {
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
    arg_prompt: &'static str,
) -> Button {
    let args: Vec<_> = args.into_iter().collect();
    button(key, description, move |mut ctx: Context| {
        let result = prompt_arg(&mut ctx, arg_prompt);
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
