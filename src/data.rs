use std::{process::Stdio, sync::Arc};

use anyhow::Context;

use crate::command_line::{Arg, ArgOrder, ArgValue, CommandLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub &'static str);

#[derive(Debug, Clone)]
pub enum Action {
    Batch(Vec<Action>),
    Add(Arg),
    Toggle(Arg),
    Popup(Page),
    Run { exit: bool },
    Escape,
    ToggleCmd,
    RunHidingUi(Callback),
}

#[derive(Debug, Clone)]
pub struct Button {
    pub key: Keybind,
    pub description: String,
    pub action: Action,
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

#[derive(Clone)]
pub struct Callback(pub Arc<dyn Fn() -> anyhow::Result<Action>>);

impl Callback {
    pub fn new(f: impl Fn() -> anyhow::Result<Action> + 'static) -> Self {
        Callback(Arc::new(f))
    }
}

impl std::fmt::Debug for Callback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback").finish_non_exhaustive()
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
                        Button { key: Keybind($key), description: $desc.into(), action: $act, },
                    )+]
                },
            )+]
        }
    };
}

fn toggle_flag(flag: &str) -> Action {
    Action::Toggle(Arg::switch(flag))
}

fn run_with_flags_esc(args: Vec<Arg>) -> Action {
    let mut actions = Vec::new();
    for arg in args {
        actions.push(Action::Add(arg));
    }
    actions.push(Action::Run { exit: false });
    actions.push(Action::Escape);
    Action::Batch(actions)
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
        .arg(format!("zoxide query -i"))
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let mut output_text = String::from_utf8(output.stdout)?;
    output_text.truncate(output_text.trim_end().len());
    Ok(output_text)
}

pub fn program(name: &str, page: Page) -> Action {
    Action::Batch(vec![Action::Popup(page), Action::Add(Arg::program(name))])
}

pub fn top() -> Program {
    Program {
        base: CommandLine::from_iter([]),
        start: page! {
            "Home"

            group "Commands":
                "b" "Build" => toggle_flag("todo_build"),
                "g" "Git" => program("git", git()),
                "c" "Change Directory" => Action::RunHidingUi(Callback::new(|| {
                    let dir = select_zoxide()?;
                    std::env::set_current_dir(&dir)?;
                    std::env::set_var("PWD", dir);
                    Ok(Action::Batch(vec![]))
                })),
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
            "p" "origin/master" => Action::Run { exit: false },
            "u" "upstream" => Action::Run { exit: false },
            "e" "elsewhere" => Action::RunHidingUi(Callback::new(|| {
                let branch = select_branch("--remote")?;
                let (remote, branch) = branch.split_once("/").context("branch should be remote branch")?;
                let arg = Arg::new(
                    ArgOrder::POSITIONAL,
                    ArgValue::Multi(vec![
                        ArgValue::Simple(remote.to_string()),
                        ArgValue::Simple(format!("HEAD:{branch}"))
                    ])
                );
                Ok(Action::Batch(vec![Action::Add(arg), Action::Run { exit: false }, Action::Escape]))
            })),
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
            "c" "Commit" => run_with_flags_esc(vec![]),
            "e" "Extend" => run_with_flags_esc(vec![Arg::switch("--no-edit"), Arg::switch("--amend")]),
            "w" "Reword" => run_with_flags_esc(vec![Arg::switch("--amend"), Arg::switch("--only"), Arg::switch("--allow-empty")]),
            "a" "Amend" => run_with_flags_esc(vec![Arg::switch("--amend")]),
            "f" "Fixup" => Action::Run { exit: false },
            "F" "Instant Fixup" => Action::Run { exit: false },
    };

    page! {
        "Git"

        group "Git Commands":
            "c" "Commit" => Action::Batch(vec![Action::Popup(commit), Action::Add(Arg::subcommand("commit"))]),
            "p" "Push" => Action::Batch(vec![Action::Popup(push), Action::Add(Arg::subcommand("push"))]),
    }
}
