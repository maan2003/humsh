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
}

impl Config {
    pub fn read(path: impl AsRef<Path>) -> anyhow::Result<Config> {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn into_page(self) -> data::Page {
        data::Page {
            description: String::from("Config"),
            groups: vec![data::Group {
                description: String::from("Commands"),
                buttons: self
                    .commands
                    .into_iter()
                    .map(move |x| {
                        data::button(&x.key, x.desc, move |mut ctx: Context| {
                            ctx.leave_ui()?;
                            ctx.run_command(
                                std::process::Command::new("bash").arg("-c").arg(&x.command),
                            )?
                            .wait()?;
                            Ok(())
                        })
                    })
                    .collect(),
            }],
        }
    }
}
