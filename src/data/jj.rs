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

fn jj_select_rev(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<String>> {
    move |ctx| {
        ctx.leave_ui()?;
        let output = shell_cmd(format!(
            "jj log -r '::' --no-graph --color=always | fzf --ansi --multi --prompt '{arg}'"
        ))
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
}

fn jj_select_branch(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<String>> {
    move |ctx| {
        ctx.leave_ui()?;
        let output = shell_cmd(format!(
            "jj branch list --color=always | fzf --ansi --multi --prompt '{arg}'"
        ))
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
}

fn jj_prompt_rev(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let revs = jj_select_rev(arg)(ctx)?;
        let mut args = vec![];
        for rev in revs {
            args.push(Arg::switch(format!("{arg}{rev}")));
        }
        Ok(args)
    }
}

fn jj_prompt_branch(arg: &'static str) -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let revs = jj_select_branch(arg)(ctx)?;
        let mut args = vec![];
        for rev in revs {
            args.push(Arg::switch(format!("{arg}{rev}")));
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
                        jj_prompt_rev("--change="),
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
            subcommand_page_button(
                "D",
                "Describe",
                ["desc"],
                [
                    flag_button("a", "Reset Author", "--reset-author"),
                    flag_button("E", "No Edit", "--no-edit"),
                ],
                [exec_button_arg_prompt(
                    "D",
                    "Describe",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev(""),
                )],
            ),
            exec_button("l", "Log", [Arg::subcommand("log")], PageAction::None),
            subcommand_page_button(
                "n",
                "New",
                ["new"],
                [
                    flag_button("a", "After", "--insert-after"),
                    flag_button("b", "Before", "--insert-before"),
                    flag_button("E", "No edit", "--no-edit"),
                ],
                [exec_button_arg_prompt(
                    "n",
                    "New",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev(""),
                )],
            ),
            subcommand_page_button(
                "S",
                "Squash",
                ["squash"],
                [flag_button("i", "Interactive", "--interactive")],
                [exec_button_arg_prompt(
                    "S",
                    "Squash",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("--revision="),
                )],
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
                    prompt_button("b", "Branch", "--branch", jj_select_branch("--branch=")),
                    prompt_button("s", "Source", "--source", jj_select_rev("--source=")),
                ],
                [exec_button_arg_prompt(
                    "r",
                    "Rebase",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("--destination="),
                )],
            ),
            subcommand_page_button(
                "m",
                "Move",
                ["move"],
                [flag_button("i", "Interactive", "--interactive")],
                [exec_button_arg_prompt2(
                    "m",
                    "Move",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("--from="),
                    jj_prompt_rev("--to="),
                )],
            ),
            subcommand_page_button(
                "b",
                "Branch",
                [],
                [],
                [
                    button("c", "Create", |mut ctx| {
                        let revs = jj_prompt_rev("--revision=")(&mut ctx)?;
                        let mut command_line = ctx.command_line().clone();
                        command_line.args.extend(revs);
                        let branch = ctx.read_input("Branches")?;
                        command_line.add_arg(Arg::subcommands(["branch", "create"]));
                        command_line.add_arg(Arg::positional(branch));
                        ctx.run_command_line_other(&command_line)?;
                        Ok(())
                    }),
                    button("s", "Set", |mut ctx| {
                        let revs = jj_prompt_rev("--revision=")(&mut ctx)?;
                        let branch = jj_prompt_branch("")(&mut ctx)?;
                        let mut command_line = ctx.command_line().clone();
                        command_line.args.extend(revs);
                        command_line.args.extend(branch);
                        command_line.add_arg(Arg::subcommands(["branch", "set"]));
                        command_line.add_arg(Arg::switch("--allow-backwards"));
                        ctx.run_command_line_other(&command_line)?;
                        Ok(())
                    }),
                    button("d", "Delete", |mut ctx| {
                        let branch = jj_prompt_branch("")(&mut ctx)?;
                        let mut command_line = ctx.command_line().clone();
                        command_line.args.extend(branch);
                        command_line.add_arg(Arg::subcommands(["branch", "delete"]));
                        ctx.run_command_line_other(&command_line)?;
                        Ok(())
                    }),
                ],
            ),
            exec_button_arg_prompt(
                "e",
                "Edit",
                [Arg::subcommand("edit")],
                PageAction::None,
                jj_prompt_rev(""),
            ),
            button("s", "Refresh status", |mut ctx| {
                ctx.currrent_page_mut().refresh_status()
            }),
        ],
    )])
    .with_status(jj_status);

    Ok(page)
}
