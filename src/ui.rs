use crate::command_line::{Arg, ArgOrder, CommandLine};

pub struct Ui {
    command_line: CommandLine,
    active_group: Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keybind(&'static str);

#[derive(Debug, Clone)]
pub enum Action {
    Toggle(Arg),
    Popup(Group),
    Run,
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
    pub components: Vec<Component>,
}

#[derive(Debug, Clone)]
pub enum Component {
    SubGroup(Group),
    Button(Button),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub base: CommandLine,
    pub start: Group,
}

pub fn git_push() -> Program {
    Program {
        base: CommandLine::from_iter([
            Arg::new(ArgOrder::PROGRAM, "git"),
            Arg::new(ArgOrder::SUBCOMMAND, "push"),
        ]),
        start: Group {
            description: String::from("Commit"),
            components: vec![
                Component::Button(Button {
                    key_prefix: Some(Keybind('-')),
                    key: Keybind('f'),
                    description: String::from("Force with lease"),
                    action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--force-with-lease")),
                }),
                Component::Button(Button {
                    key_prefix: Some(Keybind('-')),
                    key: Keybind('F'),
                    description: String::from("Force"),
                    action: Action::Toggle(Arg::new(ArgOrder::FLAG, "--force")),
                }),
                Component::Button(Button {
                    key_prefix: None,
                    key: Keybind('p'),
                    description: String::from("Push"),
                    action: Action::Run,
                }),
            ],
        },
    }
}
