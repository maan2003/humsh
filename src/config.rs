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

    pub fn into_group(self, desc: impl Into<String>) -> data::Group {
        let desc = desc.into();
        data::Group {
            description: desc,
            buttons: self
                .commands
                .into_iter()
                .map(move |x| {
                    data::button(&x.key, x.desc, move |mut ctx: Context| {
                        let mut command = std::process::Command::new("bash");
                        command.arg("-c").arg(&x.command);
                        if x.term {
                            ctx.run_command_new_term(&mut command)?;
                        } else {
                            ctx.leave_ui()?;
                            ctx.show_cmd()?;
                            ctx.hint_running_command(&x.command)?;
                            ctx.run_command_in_foreground(&mut command)?;
                        }
                        Ok(())
                    })
                })
                .collect(),
        }
    }
}
