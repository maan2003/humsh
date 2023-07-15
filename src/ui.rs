use crate::command_line::{Arg, ArgOrder, CommandLine};

pub struct Ui {
    command_line: CommandLine,
    active_group: Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(char);

#[derive(Debug, Clone)]
pub enum ButtonKind {
    Toggle(Arg),
    Popup(Group),
    Run,
}

#[derive(Debug, Clone)]
pub struct Button {
    pub key_prefix: Option<Key>,
    pub key: Key,
    pub description: String,
    pub kind: ButtonKind,
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
                    key_prefix: Some(Key('-')),
                    key: Key('f'),
                    description: String::from("Force with lease"),
                    kind: ButtonKind::Toggle(Arg::new(ArgOrder::FLAG, "--force-with-lease")),
                }),
                Component::Button(Button {
                    key_prefix: Some(Key('-')),
                    key: Key('F'),
                    description: String::from("Force"),
                    kind: ButtonKind::Toggle(Arg::new(ArgOrder::FLAG, "--force")),
                }),
                Component::Button(Button {
                    key_prefix: None,
                    key: Key('p'),
                    description: String::from("Push"),
                    kind: ButtonKind::Run,
                }),
            ],
        },
    }
}
