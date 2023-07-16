use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub args: BTreeSet<Arg>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arg {
    pub order: ArgOrder,
    pub value: String,
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
impl CommandLine {
    pub fn from_args(args: BTreeSet<Arg>) -> CommandLine {
        CommandLine { args }
    }

    pub fn toggle_arg(&mut self, arg: Arg) {
        if !self.args.remove(&arg) {
            self.args.insert(arg);
        }
    }

    pub fn to_std(&self) -> std::process::Command {
        let mut iter = self.args.iter();
        let program = iter.next().expect("must have a program name");
        let mut cmd = std::process::Command::new(program.value.clone());
        cmd.args(iter.map(|x| x.value.clone()));
        cmd
    }
}

impl FromIterator<Arg> for CommandLine {
    fn from_iter<I: IntoIterator<Item = Arg>>(iter: I) -> CommandLine {
        CommandLine::from_args(BTreeSet::from_iter(iter))
    }
}

impl Arg {
    pub fn new(order: ArgOrder, value: impl Into<String>) -> Self {
        Self {
            order,
            value: value.into(),
        }
    }
}
