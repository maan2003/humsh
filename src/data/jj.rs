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

fn jj_select_rev(ctx: &mut Context) -> anyhow::Result<Vec<String>> {
    ctx.leave_ui()?;
    let output = shell_cmd("jj log -r '::' --no-graph --color=always | fzf --ansi --multi")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let output_text = String::from_utf8(output.stdout)?;
    let revs = output_text
        .trim()
        .lines()
        .map(|x| {
            x.get(0..x.find(' ').context("invalid output")?)
                .context("invalid output")
                .map(|x| x.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(revs)
}

fn jj_select_branch(ctx: &mut Context) -> anyhow::Result<Vec<String>> {
    ctx.leave_ui()?;
    let output = shell_cmd("jj branch list --color=always | fzf --ansi --multi")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;
    let output_text = String::from_utf8(output.stdout)?;
    let revs = output_text
        .trim()
        .lines()
        .map(|x| {
            x.get(0..x.find(':').context("invalid output")?)
                .context("invalid output")
                .map(|x| x.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(revs)
}

fn jj_prompt_rev(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let revs = jj_select_rev(ctx)?;
        let mut args = vec![];
        for rev in revs {
            args.push(Arg::switch(format!("--{arg}={rev}")));
        }
        Ok(args)
    }
}

fn jj_prompt_branch(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let revs = jj_select_branch(ctx)?;
        let mut args = vec![];
        for rev in revs {
            args.push(Arg::switch(format!("--{arg}={rev}")));
        }
        Ok(args)
    }
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
                ],
                [
                    exec_button("p", "Push", [], PageAction::Pop),
                    exec_button_arg_prompt(
                        "c",
                        "Change",
                        [],
                        PageAction::Pop,
                        jj_prompt_rev("change"),
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
            subcommand_page_button(
                "S",
                "Squash",
                ["squash"],
                [flag_button("i", "Interactive", "--interactive")],
                [
                    exec_button("S", "Squash", [], PageAction::Pop),
                    exec_button_arg_prompt(
                        "r",
                        "Revision",
                        [],
                        PageAction::Pop,
                        jj_prompt_rev("revision"),
                    ),
                ],
            ),
            subcommand_page_button(
                "c",
                "Commit",
                ["commit"],
                [flag_button("i", "Interactive", "--interactive")],
                [exec_button("c", "Commit", [], PageAction::Pop)],
            ),
            subcommand_page_button(
                "r",
                "Rebase",
                ["rebase"],
                [
                    flag_button("i", "Interactive", "--interactive"),
                    flag_button("e", "Skip Empty", "--skip-empty"),
                    prompt_button("b", "Branch", "--branch", jj_select_branch),
                    prompt_button("s", "Source", "--source", jj_select_rev),
                ],
                [exec_button_arg_prompt(
                    "r",
                    "Rebase",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("destination"),
                )],
            ),
            button("s", "Refresh status", |mut ctx| {
                ctx.currrent_page_mut().refresh_status()
            }),
        ],
    )])
    .with_status(jj_status);

    Ok(page)
}
