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
            if !output.status.success() {
                bail!("direnv failed");
            }
            let buf = String::from_utf8(output.stdout)?;
            let env = buf
                .split_terminator('\0')
                .flat_map(|x| x.split_once('='))
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect();
            self.child_or_env = ChildOrEnv::Env(env);
        };
        let ChildOrEnv::Env(e) = &self.child_or_env else { unreachable!("child case handled above"); };
        Ok(e)
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
