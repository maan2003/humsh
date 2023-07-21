use std::process::{self, Child, Stdio};
use std::{mem, path::PathBuf};

use anyhow::{bail, Context};

pub struct Direnv {
    dir: PathBuf,
    child_or_env: ChildOrEnv,
}

enum ChildOrEnv {
    Child(Child),
    Env(Vec<(String, String)>),
    Error(String),
}

impl Direnv {
    pub fn new(dir: PathBuf) -> anyhow::Result<Self> {
        let child = process::Command::new("direnv")
            .arg("exec")
            .arg(&dir)
            .arg("env")
            .arg("-0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            // TODO: handle failures better
            .context("running direnv")?;

        Ok(Self {
            child_or_env: ChildOrEnv::Child(child),
            dir,
        })
    }

    fn env(&mut self) -> anyhow::Result<&[(String, String)]> {
        if let ChildOrEnv::Child(c) = mem::replace(&mut self.child_or_env, ChildOrEnv::Env(vec![]))
        {
            let output = c.wait_with_output()?;
            self.child_or_env = if output.status.success() {
                let buf = String::from_utf8(output.stdout)?;
                let env = buf
                    .split_terminator('\0')
                    .flat_map(|x| x.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
                    .collect();
                ChildOrEnv::Env(env)
            } else {
                ChildOrEnv::Error(
                    String::from_utf8(output.stderr)
                        .unwrap_or_else(|_| String::from("<invalid utf8>")),
                )
            };
        };
        match &self.child_or_env {
            ChildOrEnv::Child(_) => unreachable!(),
            ChildOrEnv::Env(e) => Ok(e),
            ChildOrEnv::Error(e) => bail!("{e}"),
        }
    }

    pub fn hook(&mut self, cmd: &mut process::Command) -> anyhow::Result<()> {
        let env = self.env()?;
        cmd.envs(env.iter().cloned());
        Ok(())
    }
}

impl Drop for Direnv {
    fn drop(&mut self) {
        if let ChildOrEnv::Child(mut c) =
            mem::replace(&mut self.child_or_env, ChildOrEnv::Env(vec![]))
        {
            c.kill().expect("unable to kill direnv");
        }
    }
}
