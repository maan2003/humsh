use std::process::ExitStatus;

use anyhow::bail;

pub trait CheckExitStatus {
    fn check_exit_status(self) -> anyhow::Result<Self>
    where
        Self: Sized;
}

impl CheckExitStatus for std::process::Output {
    fn check_exit_status(self) -> anyhow::Result<Self> {
        exit_status_to_error(self.status)?;
        Ok(self)
    }
}

impl CheckExitStatus for std::process::ExitStatus {
    fn check_exit_status(self) -> anyhow::Result<Self> {
        exit_status_to_error(self)?;
        Ok(self)
    }
}

pub fn exit_status_to_error(exit_code: ExitStatus) -> anyhow::Result<()> {
    if exit_code.success() {
        Ok(())
    } else {
        bail!("exit code {}", exit_code.code().unwrap_or(-1));
    }
}
