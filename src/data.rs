use crate::command_line::{Arg, CommandLine};

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

pub fn git_push() -> Program {
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
            "p" "elsewhere" => Action::Run { exit: false },
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

    Program {
        base: CommandLine::from_iter([Arg::program("git")]),
        start: page! {
            "Git"

            group "Git Commands":
                "c" "Commit" => Action::Batch(vec![Action::Popup(commit), Action::Add(Arg::subcommand("commit"))]),
                "p" "Push" => Action::Batch(vec![Action::Popup(push), Action::Add(Arg::subcommand("push"))]),
        },
    }
}
