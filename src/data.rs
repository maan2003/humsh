use crate::command_line::{Arg, ArgOrder, CommandLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub &'static str);

#[derive(Debug, Clone)]
pub enum Action {
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
    Action::Toggle(Arg::new(ArgOrder::FLAG, flag))
}

pub fn git_push() -> Program {
    Program {
        base: CommandLine::from_iter([
            Arg::new(ArgOrder::PROGRAM, "git"),
            Arg::new(ArgOrder::SUBCOMMAND, "push"),
        ]),
        start: page! {
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
        },
    }
}
