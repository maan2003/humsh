use std::{process::Stdio, sync::Arc};

use anyhow::Context as _;

use crate::command_line::{Arg, ArgOrder, ArgValue, CommandLine};
use crate::ui::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub &'static str);

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

macro_rules! page {
    ($pdesc:literal $(group $group:literal:
        $($key:literal $desc:literal => $act:expr),+ $(,)?)+
    ) => {
        Page {
            description: $pdesc.into(),
            groups: vec![ $(
                Group {
                    description: $group.into(),
                    buttons: vec![$(
                        Button { key: Keybind($key), description: $desc.into(), callback: Arc::new($act), },
                    )+]
                },
            )+]
        }
    };
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

fn select_zoxide() -> anyhow::Result<String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg("zoxide query -l | fzf")
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
        ctx.command_line_mut().add_arg(self.0.clone());
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self as _
    }
}
fn toggle_flag(flag: &str) -> ToggleFlag {
    ToggleFlag(Arg::switch(flag))
}

pub fn top() -> Program {
    Program {
        base: CommandLine::from_iter([]),
        start: page! {
            "Home"

            group "Commands":
                "b" "Build" => toggle_flag("todo_build"),
                "g" "Git" => |mut ctx: Context| {
                    ctx.push_page(git());
                    ctx.command_line_mut().add_arg(Arg::program("git"));
                    Ok(())
                },
                "e" "Edit" => |_: Context| {
                    todo!();
                },
                "c" "Change Directory" => |_ctx: Context| {
                    let dir = select_zoxide()?;
                    std::env::set_current_dir(&dir)?;
                    std::env::set_var("PWD", dir);
                    Ok(())
                },
                "s" "Shell Command" => |mut ctx: Context| {
                    ctx.leave_ui()?;
                    std::process::Command::new("fish").spawn()?.wait()?;
                    ctx.enter_ui()?;
                    Ok(())
                },
        },
    }
}
pub fn git() -> Page {
    let push = page! {
        "Git Push"

        group "Arguments":
            "-f" "Force with lease" => toggle_flag("--force-with-lease"),
            "-F" "Force" => toggle_flag("--force"),
            "-h" "Disable hooks" => toggle_flag("--no-verify"),
            "-n" "Dry run" => toggle_flag("--dry-run"),

        group "Push to":
            "p" "origin/master" => |mut ctx: Context| ctx.run_command_line(),
            "u" "upstream" => |mut ctx: Context| ctx.run_command_line(),
            "e" "elsewhere" => |mut ctx: Context| {
                let branch = select_branch("--remote")?;
                let (remote, branch) = branch.split_once('/').context("branch should be remote branch")?;
                let arg = Arg::new(
                    ArgOrder::POSITIONAL,
                    ArgValue::Multi(vec![
                        ArgValue::Simple(remote.to_string()),
                        ArgValue::Simple(format!("HEAD:{branch}"))
                    ])
                );
                ctx.command_line_mut().add_arg(arg);
                ctx.run_command_line()?;
                Ok(())
            },
    };

    let run_with_args = |args: Vec<Arg>| {
        move |mut ctx: Context| {
            for arg in &args {
                ctx.command_line_mut().add_arg(arg.clone());
            }
            ctx.run_command_line()?;
            Ok(())
        }
    };
    let commit = page! {
        "Git Commit"

        group "Arguments":
            "-a" "Stage all modified and deleted files" => toggle_flag("--all"),
            "-e" "Allow empty commit" => toggle_flag("--allow-empty"),
            "-v" "Show diff of changes to be committed" => toggle_flag("--verbose"),
            "-n" "Disable hooks" => toggle_flag("--no-verify"),
            "-R" "Claim authorship and reset author date" => toggle_flag("--reset-author"),
            // "-A" "Override the author" => toggle_flag("--author"),
            "-s" "Add Signed-off-by line" => toggle_flag("--signoff"),
            // "-C" "Reuse commit message" => toggle_flag("--reuse-message"),

        group "Actions":
            "c" "Commit" => run_with_args(vec![]),
            "e" "Extend" => run_with_args(vec![Arg::switch("--no-edit"), Arg::switch("--amend")]),
            "w" "Reword" => run_with_args(vec![Arg::switch("--amend"), Arg::switch("--only"), Arg::switch("--allow-empty")]),
            "a" "Amend" => run_with_args(vec![Arg::switch("--amend")]),
            "f" "Fixup" => run_with_args(vec![]),
            "F" "Instant Fixup" => run_with_args(vec![]),
    };

    page! {
        "Git"

        group "Git Commands":
            "c" "Commit" => move |mut ctx: Context| {
                ctx.push_page(commit.clone());
                ctx.command_line_mut().add_arg(Arg::subcommand("commit"));
                Ok(())
            },
            "p" "Push" => move |mut ctx: Context| {
                ctx.push_page(push.clone());
                ctx.command_line_mut().add_arg(Arg::subcommand("push"));
                Ok(())
            },
    }
}
