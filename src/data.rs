mod cp;
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
    pub callback: Arc<dyn Callback>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub description: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub status: Option<String>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub base: CommandLine,
    pub start: Page,
}

pub trait Callback {
    fn call(&self, ctx: Context<'_, '_>) -> anyhow::Result<()>;
    fn as_any(&self) -> &dyn std::any::Any;
}

impl std::fmt::Debug for dyn Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback").finish_non_exhaustive()
    }
}

impl<F> Callback for F
where
    F: Fn(Context) -> anyhow::Result<()> + 'static,
{
    fn call(&self, ctx: Context<'_, '_>) -> anyhow::Result<()> {
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
            groups: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
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

impl Callback for ToggleFlag {
    fn call(&self, mut ctx: Context<'_, '_>) -> anyhow::Result<()> {
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
    callback: impl Callback + 'static,
) -> Button {
    Button {
        key: Keybind(key.into()),
        description: description.into(),
        callback: Arc::new(callback),
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
            ctx.push_page(git()?);
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
    if shell_context.is_cp() {
        builtin_buttons.push(button(
            "C",
            "Competitive programming",
            |mut ctx: Context| {
                let current_dir = std::env::current_dir()?;
                let cp = cp::Cp::new(current_dir)?;
                ctx.push_page(cp::cp_page(Arc::new(Mutex::new(cp)))?);
                Ok(())
            },
        ));
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

fn jj_status() -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("status")
        .arg("--color=always")
        .output()?;
    Ok(String::from_utf8(output.stdout)?)
}

pub fn git() -> anyhow::Result<Page> {
    let push = page([
        group(
            "Arguments",
            [
                flag_button("-d", "Deleted", "--deleted"),
                flag_button("-n", "Dry run", "--dry-run"),
            ],
        ),
        group(
            "Push",
            [
                button("p", "push", |mut ctx: Context| {
                    ctx.run_command_line()?;
                    ctx.show_cmd()?;
                    ctx.enter_ui()?;
                    ctx.pop_page();
                    Ok(())
                }),
                button("c", "change", |mut ctx: Context| {
                    let input = ctx.read_input()?;
                    ctx.command_line_mut()
                        .add_arg(Arg::switch(format!("--change={input}")));
                    ctx.run_command_line()?;
                    ctx.show_cmd()?;
                    Ok(())
                }),
            ],
        ),
    ]);

    let run_with_args = |args: Vec<Arg>| {
        move |mut ctx: Context| {
            for arg in &args {
                ctx.command_line_mut().add_arg(arg.clone());
            }
            ctx.run_command_line()?;
            Ok(())
        }
    };
    let commit = page([
        group(
            "Arguments",
            [
                flag_button("-a", "Stage all modified and deleted files", "--all"),
                flag_button("-e", "Allow empty commit", "--allow-empty"),
                flag_button("-v", "Show diff of changes to be committed", "--verbose"),
                flag_button("-n", "Disable hooks", "--no-verify"),
                flag_button(
                    "-R",
                    "Claim authorship and reset author date",
                    "--reset-author",
                ),
                // flag_button("-A", "Override the author", "--author"),
                flag_button("-s", "Add Signed-off-by line", "--signoff"),
                // flag_button("-C", "Reuse commit message", "--reuse-message"),
            ],
        ),
        group(
            "Actions",
            [
                button("c", "Commit", run_with_args(vec![])),
                button(
                    "e",
                    "Extend",
                    run_with_args(vec![Arg::switch("--no-edit"), Arg::switch("--amend")]),
                ),
                button(
                    "w",
                    "Reword",
                    run_with_args(vec![
                        Arg::switch("--amend"),
                        Arg::switch("--only"),
                        Arg::switch("--allow-empty"),
                    ]),
                ),
                button("a", "Amend", run_with_args(vec![Arg::switch("--amend")])),
                button("f", "Fixup", run_with_args(vec![])),
                button("F", "Instant Fixup", run_with_args(vec![])),
            ],
        ),
    ]);

    let fetch = page([group(
        "Fetch from",
        [
            button(
                "p",
                "origin",
                run_with_args(vec![Arg::positional("origin")]),
            ),
            button(
                "u",
                "upstream",
                run_with_args(vec![Arg::positional("upstream")]),
            ),
            button("f", "all", run_with_args(vec![Arg::switch("--all")])),
        ],
    )]);

    let page = page([group(
        "JJ Commands",
        [
            // button("c", "Commit", move |mut ctx: Context| {
            //     ctx.push_page(commit.clone());
            //     ctx.command_line_mut().add_arg(Arg::subcommand("commit"));
            //     // TODO: enable this later
            //     // ctx.command_line_mut().add_arg(Arg::switch("--verbose"));
            //     Ok(())
            // }),
            button("p", "Push", move |mut ctx: Context| {
                ctx.push_page(push.clone());
                ctx.command_line_mut().add_arg(Arg::new(
                    ArgOrder::SUBCOMMAND,
                    ArgValue::Multi(vec!["git".to_string(), "push".to_string()]),
                ));
                Ok(())
            }),
            button("f", "Fetch", move |mut ctx: Context| {
                ctx.push_page(fetch.clone());
                ctx.command_line_mut().add_arg(Arg::new(
                    ArgOrder::SUBCOMMAND,
                    ArgValue::Multi(vec!["git".to_string(), "fetch".to_string()]),
                ));
                ctx.run_command_line()?;
                ctx.show_cmd()?;
                ctx.pop_page();
                Ok(())
            }),
            button("d", "Diff", |mut ctx: Context| {
                ctx.run_command_in_foreground(Command::new("jj").arg("diff"))?;
                Ok(())
            }),
            button("D", "Describe", |mut ctx: Context| {
                ctx.run_command_in_foreground(Command::new("jj").arg("desc"))?;
                Ok(())
            }),
            button("l", "Log", |mut ctx: Context| {
                ctx.run_command_in_foreground(Command::new("jj").arg("log"))?;
                ctx.show_cmd()?;
                Ok(())
            }),
            button("n", "New", |mut ctx: Context| {
                ctx.run_command_in_foreground(Command::new("jj").arg("new"))?;
                Ok(())
            }),
        ],
    )])
    .with_status(jj_status()?);

    Ok(page)
}
