use anyhow::Context as _;

use super::*;

fn jj_status() -> anyhow::Result<String> {
    let output = Command::new("jj")
        .arg("status")
        .arg("--color=always")
        .output()?;
    Ok(String::from_utf8(output.stdout)?)
}

fn shell_cmd(bash_cmd: impl Into<String>) -> Command {
    let mut cmd = std::process::Command::new("bash");
    cmd.arg("-c");
    cmd.arg(bash_cmd.into());
    cmd
}

fn jj_select_rev(ctx: &mut Context) -> anyhow::Result<String> {
    ctx.leave_ui()?;
    let output = shell_cmd("jj log -r '::' --no-graph --color=always | fzf --ansi")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let mut output_text = String::from_utf8(output.stdout)?;
    output_text.truncate(output_text.find(' ').context("invalid output")?);
    Ok(output_text)
}

pub fn jj() -> anyhow::Result<Page> {
    let page = page([group(
        "Commands",
        [
            subcommand_page_button(
                "p",
                "Push",
                ["git", "push"],
                [
                    flag_button("d", "Deleted", "--deleted"),
                    flag_button("n", "Dry run", "--dry-run"),
                    prompt_button("r", "Revision", "--revision", jj_select_rev),
                ],
                [
                    exec_button("p", "Push", [], PageAction::Pop),
                    exec_button_arg_prompt(
                        "c",
                        "Change",
                        [],
                        PageAction::Pop,
                        "change",
                        jj_select_rev,
                    ),
                ],
            ),
            exec_button(
                "f",
                "Fetch",
                [Arg::subcommands(["git", "fetch"])],
                PageAction::None,
            ),
            exec_button("d", "Diff", [Arg::subcommand("diff")], PageAction::None),
            exec_button("D", "Describe", [Arg::subcommand("desc")], PageAction::None),
            exec_button("l", "Log", [Arg::subcommand("log")], PageAction::None),
            subcommand_page_button(
                "n",
                "New",
                ["new"],
                [
                    flag_button("a", "After", "--insert-after"),
                    flag_button("b", "Before", "--insert-before"),
                ],
                [exec_button("n", "New", [], PageAction::Pop)],
            ),
            exec_button("S", "Squash", [Arg::subcommand("squash")], PageAction::None),
            button("s", "Refresh status", |mut ctx| {
                ctx.currrent_page_mut().refresh_status()
            }),
        ],
    )])
    .with_status(jj_status);

    Ok(page)
}
