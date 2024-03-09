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

#[derive(Clone, Copy, Debug)]
enum RevSelector {
    All,
    Mutable,
    NotInTrunk,
}

fn jj_select_rev(
    arg: &'static str,
    revs: RevSelector,
) -> impl Fn(&mut Context) -> anyhow::Result<Vec<String>> {
    let rev = match revs {
        RevSelector::All => "::",
        RevSelector::Mutable => ":: & ~::immutable_heads()",
        RevSelector::NotInTrunk => "trunk()..",
    };
    move |ctx| {
        ctx.leave_ui()?;
        let output = shell_cmd(format!(
            "jj log -r '{rev}' --no-graph --color=always | fzf --ansi --multi --prompt '{arg}' --tiebreak=begin,index"
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
            "jj branch list --color=always | fzf --ansi --multi --prompt '{arg}' --tiebreak=begin,index"
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

fn jj_prompt_rev(
    arg: &'static str,
    revs: RevSelector,
) -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let revs = jj_select_rev(arg, revs)(ctx)?;
        let mut args = vec![];
        for rev in revs {
            args.push(Arg::switch(format!("{arg}{rev}")));
        }
        Ok(args)
    }
}

fn jj_prompt_branch_name() -> impl Fn(&mut Context) -> anyhow::Result<Vec<Arg>> {
    move |ctx| {
        let branch = ctx.read_input("Branches")?;
        Ok(vec![Arg::positional(branch)])
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
                    flag_button("t", "Tracked", "--tracked"),
                    flag_button("a", "All", "--all"),
                    prompt_button("b", "Branch", "--branch", jj_select_branch("--branch=")),
                ],
                [
                    exec_button("p", "Push", [], PageAction::Pop),
                    exec_button_arg_prompt(
                        "c",
                        "Change",
                        [],
                        PageAction::Pop,
                        jj_prompt_rev("--change=", RevSelector::NotInTrunk),
                    ),
                ],
            ),
            exec_button(
                "f",
                "Fetch",
                [Arg::subcommands(["git", "fetch"])],
                PageAction::None,
            ),
            subcommand_page_button(
                "d",
                "Describe",
                ["desc"],
                [
                    flag_button("a", "Reset Author", "--reset-author"),
                    flag_button("E", "No Edit", "--no-edit"),
                ],
                [exec_button_arg_prompt(
                    "d",
                    "Describe",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("", RevSelector::Mutable),
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
                    jj_prompt_rev("", RevSelector::All),
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
                    jj_prompt_rev("--revision=", RevSelector::Mutable),
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
                    prompt_button(
                        "s",
                        "Source",
                        "--source",
                        jj_select_rev("--source=", RevSelector::Mutable),
                    ),
                ],
                [exec_button_arg_prompt(
                    "r",
                    "Rebase",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("--destination=", RevSelector::All),
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
                    jj_prompt_rev("--from=", RevSelector::Mutable),
                    jj_prompt_rev("--to=", RevSelector::Mutable),
                )],
            ),
            subcommand_page_button(
                "b",
                "Branch",
                ["branch"],
                [],
                [
                    exec_button_arg_prompt2(
                        "c",
                        "Create",
                        [Arg::subcommand_order("create", 1)],
                        PageAction::Pop,
                        jj_prompt_branch_name(),
                        jj_prompt_rev("--revision=", RevSelector::All),
                    ),
                    exec_button_arg_prompt2(
                        "s",
                        "Set",
                        [
                            Arg::subcommand_order("set", 1),
                            Arg::switch("--allow-backwards"),
                        ],
                        PageAction::Pop,
                        jj_prompt_branch(""),
                        jj_prompt_rev("--revision=", RevSelector::All),
                    ),
                    exec_button_arg_prompt(
                        "d",
                        "Delete",
                        [Arg::subcommand_order("delete", 1)],
                        PageAction::Pop,
                        jj_prompt_branch(""),
                    ),
                ],
            ),
            exec_button_arg_prompt(
                "e",
                "Edit",
                [Arg::subcommand("edit")],
                PageAction::None,
                jj_prompt_rev("", RevSelector::Mutable),
            ),
            exec_button_arg_prompt(
                "s",
                "Show",
                [Arg::subcommand("show")],
                PageAction::None,
                jj_prompt_rev("", RevSelector::All),
            ),
            subcommand_page_button(
                "o",
                "Obs Log",
                ["obslog"],
                [flag_button("p", "Patch", "--patch")],
                [exec_button_arg_prompt(
                    "o",
                    "Obs Log",
                    [],
                    PageAction::Pop,
                    jj_prompt_rev("--revision=", RevSelector::All),
                )],
            ),
            exec_button_arg_prompt(
                "a",
                "Abandon",
                [Arg::subcommand("abandon")],
                PageAction::None,
                jj_prompt_rev("", RevSelector::Mutable),
            ),
        ],
    )])
    .with_status(jj_status);

    Ok(page)
}
