use std::{fs, path::Path};

use crate::{data, ui::Context};

#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct Config {
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Command {
    key: String,
    desc: String,
    command: String,

    #[serde(default)]
    term: bool,
}

impl Config {
    pub fn read(path: impl AsRef<Path>) -> anyhow::Result<Config> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn into_page(self, desc: impl Into<String>) -> data::Page {
        let desc = desc.into();
        data::Page {
            status: None,
            groups: vec![data::Group {
                description: String::from(desc),
                buttons: self
                    .commands
                    .into_iter()
                    .map(move |x| {
                        data::button(&x.key, x.desc, move |mut ctx: Context| {
                            let mut command = std::process::Command::new("bash");
                            command.arg("-c").arg(&x.command);
                            if x.term {
                                ctx.leave_ui()?;
                                ctx.run_command_new_term(&mut command)?;
                            } else {
                                ctx.run_command(&mut command)?.wait()?;
                            }
                            Ok(())
                        })
                    })
                    .collect(),
            }],
        }
    }
}
