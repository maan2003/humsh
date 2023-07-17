use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub args: BTreeSet<Arg>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArgValue {
    Simple(String),
    Valued(String, String),
    Multi(Vec<ArgValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arg {
    pub order: ArgOrder,
    pub value: ArgValue,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct ArgOrder(u64);

impl ArgOrder {
    pub const PROGRAM: ArgOrder = ArgOrder(100);
    pub const SUBCOMMAND: ArgOrder = ArgOrder(200);
    pub const FLAG: ArgOrder = ArgOrder(300);
    pub const POSITIONAL: ArgOrder = ArgOrder(400);

    pub fn custom(value: u64) -> Self {
        ArgOrder(value)
    }
}

impl ArgValue {
    pub fn add_to(&self, args: &mut Vec<String>) {
        match self {
            ArgValue::Simple(s) => args.push(s.clone()),
            ArgValue::Valued(k, v) => {
                args.push(k.clone());
                args.push(v.clone());
            }
            ArgValue::Multi(m) => {
                for a in m {
                    a.add_to(args);
                }
            }
        }
    }

    pub fn to_string(&self) -> String {
        let mut args = Vec::new();
        self.add_to(&mut args);
        args.join(" ")
    }
}

impl CommandLine {
    pub fn from_args(args: BTreeSet<Arg>) -> CommandLine {
        CommandLine { args }
    }

    pub fn add_arg(&mut self, arg: Arg) {
        self.args.insert(arg);
    }

    pub fn toggle_arg(&mut self, arg: Arg) {
        if !self.args.remove(&arg) {
            self.args.insert(arg);
        }
    }

    pub fn to_std(&self) -> std::process::Command {
        let mut iter = self.args.iter();
        let program = iter.next().expect("must have a program name");
        let program = match &program.value {
            ArgValue::Simple(s) => s.clone(),
            _ => panic!("program name must be a simple argument"),
        };
        let mut cmd = std::process::Command::new(program);
        let mut args = Vec::new();
        for arg in iter {
            arg.value.add_to(&mut args);
        }
        cmd.args(args);
        cmd
    }
}

impl FromIterator<Arg> for CommandLine {
    fn from_iter<I: IntoIterator<Item = Arg>>(iter: I) -> CommandLine {
        CommandLine::from_args(BTreeSet::from_iter(iter))
    }
}

impl Arg {
    pub fn new(order: ArgOrder, value: ArgValue) -> Self {
        Self {
            order,
            value: value.into(),
        }
    }

    pub fn switch(value: impl Into<String>) -> Self {
        Arg::new(ArgOrder::FLAG, ArgValue::Simple(value.into()))
    }

    pub fn program(value: impl Into<String>) -> Self {
        Arg::new(ArgOrder::PROGRAM, ArgValue::Simple(value.into()))
    }

    pub fn subcommand(value: impl Into<String>) -> Self {
        Arg::new(ArgOrder::SUBCOMMAND, ArgValue::Simple(value.into()))
    }

    pub fn positional(value: impl Into<String>) -> Self {
        Arg::new(ArgOrder::POSITIONAL, ArgValue::Simple(value.into()))
    }
}
