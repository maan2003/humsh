use std::{path::PathBuf, process::Stdio};

use anyhow::{bail, Context};
use once_cell::unsync::Lazy;

use crate::ui::ExternalContext;

type Envs = anyhow::Result<Vec<(String, String)>>;

#[derive(Debug)]
pub struct Direnv {
    env: Lazy<Envs, Box<dyn FnOnce() -> Envs>>,
}

impl Direnv {
    pub fn new(ctx: ExternalContext, dir: PathBuf) -> anyhow::Result<Self> {
        let (tx, rx) = flume::bounded(1);
        tokio::spawn(async move {
            // FIXME
            // FIXME: stop direnv if dropped
            let id = ctx.begin_status("Direnv Loading").await;
            let _ = tx.send(Self::background(dir).await);
            ctx.remove_status(id).await;
        });

        Ok(Self {
            env: Lazy::new(Box::new(move || {
                rx.recv().context("channel disconnected")?
            })),
        })
    }

    async fn background(dir: PathBuf) -> anyhow::Result<Vec<(String, String)>> {
        let output = tokio::process::Command::new("direnv")
            .arg("exec")
            .arg(&dir)
            .arg("env")
            .arg("-0")
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await?;

        if !output.status.success() {
            bail!("direnv failed: {}", String::from_utf8(output.stderr)?);
        }

        Ok(String::from_utf8(output.stdout)?
            .split_terminator('\0')
            .flat_map(|x| x.split_once('='))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect())
    }

    fn env(&mut self) -> anyhow::Result<&[(String, String)]> {
        match self.env.as_ref() {
            Ok(env) => Ok(env),
            Err(e) => bail!("{e}"),
        }
    }

    pub fn hook(&mut self, cmd: &mut std::process::Command) -> anyhow::Result<()> {
        let env = self.env()?;
        cmd.envs(env.iter().cloned());
        Ok(())
    }
}
