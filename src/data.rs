use std::process::Command;
use std::{process::Stdio, sync::Arc};

use anyhow::Context as _;

use crate::command_line::{Arg, ArgOrder, ArgValue, CommandLine};
use crate::ui::Context;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub String);

#[derive(Debug, Clone)]
pub struct Button {
    pub key: Keybind,
    pub description: String,
    pub callback: Arc<dyn Callback>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub description: String,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub description: String,
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
            description: String::from("empty"),
            groups: Vec::new(),
        }
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
        .arg("cat <(zoxide query -l) <(fd --follow --maxdepth 3 -t d) | fzf")
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

pub fn page(description: impl Into<String>, groups: impl Into<Vec<Group>>) -> Page {
    Page {
        description: description.into(),
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
    }
}

pub fn flag_button(key: &'static str, description: &str, flag: &str) -> Button {
    button(key, description, ToggleFlag(Arg::switch(flag)))
}

pub fn top() -> Program {
    Program {
        base: CommandLine::from_iter([]),
        start: page(
            "Home",
            [group(
                "Commands",
                [
                    button("b", "Build", toggle_flag("todo build")),
                    button("g", "Git", |mut ctx: Context| {
                        ctx.push_page(git());
                        ctx.command_line_mut().add_arg(Arg::program("git"));
                        Ok(())
                    }),
                    button("e", "Edit", |mut ctx: Context| {
                        ctx.leave_ui()?;
                        ctx.run_command(Command::new("hx").arg("."))?.wait()?;
                        Ok(())
                    }),
                    button("c", "Change Directory", |mut ctx: Context| {
                        ctx.change_dir(select_directory()?)
                    }),
                    button("s", "Shell Command", |mut ctx: Context| {
                        ctx.leave_ui()?;
                        let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
                        ctx.run_command(&mut Command::new(shell))?.wait()?;
                        Ok(())
                    }),
                ],
            )],
        ),
    }
}

pub fn git() -> Page {
    let push = page(
        "Git Push",
        [
            group(
                "Arguments",
                [
                    flag_button("-f", "Force with lease", "--force-with-lease"),
                    flag_button("-F", "Force", "--force"),
                    flag_button("-h", "Disable hooks", "--no-verify"),
                    flag_button("-n", "Dry run", "--dry-run"),
                ],
            ),
            group(
                "Push to",
                [
                    button("p", "origin/master", |mut ctx: Context| {
                        ctx.run_command_line()
                    }),
                    button("u", "upstream", |mut ctx: Context| ctx.run_command_line()),
                    button("e", "elsewhere", |mut ctx: Context| {
                        let branch = select_branch("--remote")?;
                        let (remote, branch) = branch
                            .split_once('/')
                            .context("branch should be remote branch")?;
                        let arg = Arg::new(
                            ArgOrder::POSITIONAL,
                            ArgValue::Multi(vec![
                                ArgValue::Simple(remote.to_string()),
                                ArgValue::Simple(format!("HEAD:{branch}")),
                            ]),
                        );
                        ctx.command_line_mut().add_arg(arg);
                        ctx.run_command_line()?;
                        Ok(())
                    }),
                ],
            ),
        ],
    );

    let run_with_args = |args: Vec<Arg>| {
        move |mut ctx: Context| {
            for arg in &args {
                ctx.command_line_mut().add_arg(arg.clone());
            }
            ctx.run_command_line()?;
            Ok(())
        }
    };
    let commit = page(
        "Git commit",
        [
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
        ],
    );

    page(
        "Git",
        [group(
            "Git Commands",
            [
                button("c", "Commit", move |mut ctx: Context| {
                    ctx.push_page(commit.clone());
                    ctx.command_line_mut().add_arg(Arg::subcommand("commit"));
                    Ok(())
                }),
                button("p", "Push", move |mut ctx: Context| {
                    ctx.push_page(push.clone());
                    ctx.command_line_mut().add_arg(Arg::subcommand("push"));
                    Ok(())
                }),
                button("d", "Diff", |mut ctx: Context| {
                    ctx.leave_ui()?;
                    ctx.run_command(Command::new("git").arg("diff"))?.wait()?;
                    ctx.show_cmd()?;
                    Ok(())
                }),
            ],
        )],
    )
}
