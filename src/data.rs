use crate::command_line::{Arg, ArgOrder, CommandLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(pub &'static str);

#[derive(Debug, Clone)]
pub enum Action {
    Toggle(Arg),
    Popup(Page),
    Run,
    Escape,
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

pub fn git_push() -> Program {
    Program {
        base: CommandLine::from_iter([
            Arg::new(ArgOrder::PROGRAM, "git"),
            Arg::new(ArgOrder::SUBCOMMAND, "push"),
        ]),
        start: Page {
            description: String::from("Push"),
            groups: vec![
                Group {
                    description: String::from("Arguments"),
                    buttons: vec![
                        Button {
                            key: Keybind("-f"),
                            description: String::from("Force with lease"),
                            action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--force-with-lease")),
                        },
                        Button {
                            key: Keybind("-F"),
                            description: String::from("Force"),
                            action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--force")),
                        },
                        Button {
                            key: Keybind("-h"),
                            description: String::from("Disable hooks"),
                            action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--no-verify")),
                        },
                        Button {
                            key: Keybind("-n"),
                            description: String::from("Dry run"),
                            action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--dry-run")),
                        },
                    ],
                },
                Group {
                    description: String::from("Push to "),
                    buttons: vec![
                        Button {
                            key: Keybind("p"),
                            description: String::from("origin/master"),
                            action: Action::Run,
                        },
                        Button {
                            key: Keybind("u"),
                            description: String::from("upstream"),
                            action: Action::Run,
                        },
                        Button {
                            key: Keybind("e"),
                            description: String::from("elsewhere"),
                            action: Action::Run,
                        },
                    ],
                },
            ],
        },
    }
}
