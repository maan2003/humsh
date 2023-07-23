use std::path::Path;
use std::process::Command;
use std::{process::Stdio, sync::Arc};

use anyhow::{anyhow, Context as _};

use crate::command_line::{Arg, ArgOrder, ArgValue, CommandLine};
use crate::config::Config;
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

    pub fn merge(mut self, other: Page) -> Page {
        self.groups.extend(other.groups);
        self.status = other.status.or(self.status);
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
    let builtin_page = page([group(
        "Builtin commands",
        [
            button("g", "Git", |mut ctx: Context| {
                ctx.push_page(git()?);
                ctx.command_line_mut().add_arg(Arg::program("git"));
                Ok(())
            }),
            button("e", "Edit", |mut ctx: Context| {
                ctx.leave_ui()?;
                ctx.run_command_new_term(Command::new("hx").arg("."))?;
                Ok(())
            }),
            button("c", "Change Directory", |mut ctx: Context| {
                ctx.change_dir(select_directory()?)?;
                ctx.replace_page(home_page()?);
                Ok(())
            }),
            button("s", "Shell Command", |mut ctx: Context| {
                ctx.leave_ui()?;
                let shell = std::env::var("SHELL").unwrap_or("bash".to_owned());
                ctx.run_command(&mut Command::new(shell))?.wait()?;
                Ok(())
            }),
            button("x", "Explore", |mut ctx: Context| {
                ctx.leave_ui()?;
                ctx.run_command(&mut Command::new("nnn"))?.wait()?;
                Ok(())
            }),
        ],
    )]);
    let maybe_merge = |name: &str, path, page: Page| -> anyhow::Result<Page> {
        if Path::try_exists(path)? {
            let new_page = Config::read(path)?.into_page(name);
            Ok(page.merge(new_page))
        } else {
            Ok(page)
        }
    };
    maybe_merge(
        "Project commands",
        ".humsh/commands.toml".as_ref(),
        maybe_merge(
            "User commands",
            &dirs::config_dir()
                .ok_or_else(|| anyhow!("config dir not found"))?
                .join("humsh/commands.toml"),
            builtin_page,
        )?,
    )
}

fn git_status() -> anyhow::Result<String> {
    let output = Command::new("git")
        .arg("-c")
        .arg("color.status=always")
        .arg("-c")
        .arg("advice.statusHints=false")
        .arg("status")
        .arg("--branch")
        .arg("--show-stash")
        .output()?;
    Ok(String::from_utf8(output.stdout)?)
}

pub fn git() -> anyhow::Result<Page> {
    let push = page([
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
                // TODO
                // button("u", "upstream", |mut ctx: Context| ctx.run_command_line()),
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
        "Git Commands",
        [
            button("c", "Commit", move |mut ctx: Context| {
                ctx.push_page(commit.clone());
                ctx.command_line_mut().add_arg(Arg::subcommand("commit"));
                // TODO: enable this later
                // ctx.command_line_mut().add_arg(Arg::switch("--verbose"));
                Ok(())
            }),
            button("p", "Push", move |mut ctx: Context| {
                ctx.push_page(push.clone());
                ctx.command_line_mut().add_arg(Arg::subcommand("push"));
                Ok(())
            }),
            button("f", "Fetch", move |mut ctx: Context| {
                ctx.push_page(fetch.clone());
                ctx.command_line_mut().add_arg(Arg::subcommand("fetch"));
                Ok(())
            }),
            button("d", "Diff", |mut ctx: Context| {
                ctx.leave_ui()?;
                ctx.run_command(Command::new("git").arg("diff"))?.wait()?;
                ctx.show_cmd()?;
                Ok(())
            }),
            button("l", "Log", |mut ctx: Context| {
                ctx.leave_ui()?;
                ctx.run_command(
                    Command::new("git")
                        .arg("log")
                        .arg("--format=oneline")
                        .arg("--abbrev-commit"),
                )?
                .wait()?;
                ctx.show_cmd()?;
                Ok(())
            }),
        ],
    )])
    .with_status(git_status()?);

    Ok(page)
}
