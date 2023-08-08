use crate::util::CheckExitStatus;

use super::*;

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_dir, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct ActiveState {
    active_contest: Option<String>,
    active_problem: Option<String>,
}

pub struct Cp {
    active_contest: Option<String>,
    active_problem: Option<String>,
    base_path: PathBuf,
}

const CONFIG_PATH: &str = ".cp.json";
impl Cp {
    pub fn new(base_path: PathBuf) -> anyhow::Result<Self> {
        let mut cp = Cp {
            active_contest: None,
            active_problem: None,
            base_path,
        };

        cp.load_from_disk()?;

        Ok(cp)
    }

    pub fn new_contest(&mut self, contest_name: &str) -> Result<()> {
        let contest_path = self.base_path.join(contest_name);
        create_dir_all(contest_path)?;
        Ok(())
    }

    pub fn new_problem(&mut self, problem_name: &str) -> Result<()> {
        let contest = self
            .active_contest
            .clone()
            .context("No active contest set")?;
        let problem_path = self.base_path.join(&contest).join(problem_name);
        create_dir_all(problem_path)?;
        Ok(())
    }

    pub fn set_active_contest(&mut self, contest_name: &str) -> Result<()> {
        self.active_contest = Some(contest_name.to_string());
        self.save_to_disk()?;
        Ok(())
    }

    pub fn set_active_problem(&mut self, problem_name: &str) -> Result<()> {
        self.active_problem = Some(problem_name.to_string());
        self.save_to_disk()?;
        Ok(())
    }

    pub fn current_contest(&self) -> Option<String> {
        self.active_contest.clone()
    }

    pub fn current_problem(&self) -> Option<String> {
        self.active_problem.clone()
    }

    pub fn current_problem_path(&self) -> Result<PathBuf> {
        let contest = self
            .active_contest
            .clone()
            .context("No active contest set")?;
        let problem = self
            .active_problem
            .clone()
            .context("No active problem set")?;
        Ok(self.base_path.join(contest).join(problem))
    }

    pub fn list_contests(&self) -> Result<Vec<String>> {
        let mut contests = Vec::new();
        for entry in read_dir(&self.base_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                contests.push(entry.file_name().into_string().unwrap());
            }
        }
        Ok(contests)
    }

    pub fn list_problems(&self) -> Result<Vec<String>> {
        let contest = self
            .active_contest
            .clone()
            .context("No active contest set")?;
        let path = self.base_path.join(contest);
        let mut problems = Vec::new();
        for entry in read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                problems.push(entry.file_name().into_string().unwrap());
            }
        }
        Ok(problems)
    }

    pub fn new_test_case(&mut self) -> Result<(PathBuf, PathBuf)> {
        let tests_path = self.current_problem_path()?.join("tests");
        create_dir_all(&tests_path)?;
        let id = read_dir(&tests_path)?.count() + 1;
        let test_path = tests_path.join(format!("{}", id));
        create_dir_all(&test_path)?;
        let test_in_path = test_path.join("in");
        let test_out_path = test_path.join("out");
        File::create(&test_in_path)?;
        File::create(&test_out_path)?;
        Ok((test_in_path, test_out_path))
    }

    pub fn list_test_cases(&self) -> Result<Vec<(PathBuf, PathBuf)>> {
        let test_path = self.current_problem_path()?.join("tests");
        let mut test_cases = Vec::new();
        if test_path.exists() {
            for entry in read_dir(test_path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    test_cases.push((entry.path().join("in"), entry.path().join("out")));
                }
            }
        }
        Ok(test_cases)
    }

    pub fn select_contest_with_fzf(&mut self) -> Result<()> {
        let contests = self.list_contests()?;
        let selected_contest = self.select_with_fzf("Contest: ", &contests)?;
        if !contests.contains(&selected_contest) {
            self.new_contest(&selected_contest)?;
        }
        self.set_active_contest(&selected_contest)?;
        Ok(())
    }

    pub fn select_problem_with_fzf(&mut self) -> Result<()> {
        let problems = self.list_problems()?;
        let selected_problem = self.select_with_fzf("Problem: ", &problems)?;
        if !problems.contains(&selected_problem) {
            self.new_problem(&selected_problem)?;
        }
        self.set_active_problem(&selected_problem)?;
        Ok(())
    }

    fn select_with_fzf(&self, prompt: &str, items: &[String]) -> Result<String> {
        let items_str = items.join("\n");
        let mut output = Command::new("fzf")
            .arg("--print-query")
            .arg("--prompt")
            .arg(prompt)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .context("Failed to start fzf")?;

        let mut child_stdin =
            std::io::BufWriter::new(output.stdin.take().context("Failed to open fzf stdin")?);
        write!(child_stdin, "{}", items_str)?;
        child_stdin.flush()?;
        drop(child_stdin);

        let output = output.wait_with_output()?;

        match output.status.code() {
            Some(0) => {
                let selected_output =
                    String::from_utf8(output.stdout).context("Failed to read fzf output")?;
                let selected_item = selected_output
                    .split('\n')
                    .nth(1)
                    .context("Unexpected fzf output")?;
                Ok(selected_item.trim().to_string())
            }
            Some(1) => {
                let selected_output =
                    String::from_utf8(output.stdout).context("Failed to read fzf output")?;
                let query = selected_output
                    .split('\n')
                    .nth(0)
                    .context("Unexpected fzf output")?;
                Ok(query.trim().to_string())
            }
            Some(130) => bail!("fzf was cancelled by the user"),
            _ => bail!("Unexpected exit code from fzf"),
        }
    }

    fn save_to_disk(&self) -> Result<()> {
        let file_path = self.base_path.join(CONFIG_PATH);
        let file = File::create(file_path)?;
        let state = ActiveState {
            active_contest: self.active_contest.clone(),
            active_problem: self.active_problem.clone(),
        };
        serde_json::to_writer(file, &state)?;
        Ok(())
    }

    fn load_from_disk(&mut self) -> Result<()> {
        let file_path = self.base_path.join(CONFIG_PATH);
        if let Ok(file) = File::open(file_path) {
            let state: ActiveState = serde_json::from_reader(file)?;
            self.active_contest = state.active_contest;
            self.active_problem = state.active_problem;
        }
        Ok(())
    }

    fn run_tests(&self) -> Result<()> {
        Command::new("c++")
            .arg("-g")
            .arg("-o")
            .arg("/tmp/a.out")
            .arg(self.code_path()?)
            .spawn()?
            .wait()?
            .check_exit_status()?;
        for (inp, expected_out) in self.list_test_cases()? {
            let actual_out = tempfile::NamedTempFile::new()?;
            Command::new("/tmp/a.out")
                .stdin(File::open(inp)?)
                .stdout(actual_out.as_file().try_clone()?)
                .spawn()?
                .wait()?
                .check_exit_status()?;
            Command::new("delta")
                .arg(expected_out)
                .arg(actual_out.path())
                .spawn()?
                .wait()?
                .check_exit_status()?;
        }
        Ok(())
    }

    fn code_path(&self) -> Result<PathBuf, anyhow::Error> {
        Ok(self.current_problem_path()?.join("main.cpp"))
    }
}

pub fn cp_page(cp: Arc<Mutex<Cp>>) -> anyhow::Result<Page> {
    let cp_lock = cp.lock().unwrap();
    Ok(page([group(
        "Competitive programming",
        [
            button("c", "Context", {
                let cp = cp.clone();
                move |mut ctx: Context| {
                    ctx.leave_ui()?;
                    cp.lock().unwrap().select_contest_with_fzf()?;
                    let page = cp_page(cp.clone())?;
                    ctx.replace_page(page);
                    Ok(())
                }
            })
            .with_hint(cp_lock.current_contest()),
            button("p", "Problem", {
                let cp = cp.clone();
                move |mut ctx: Context| {
                    ctx.leave_ui()?;
                    {
                        let mut cp = cp.lock().unwrap();
                        if cp.current_contest().is_none() {
                            cp.select_contest_with_fzf()?;
                        }
                        cp.select_problem_with_fzf()?;
                    }
                    let page = cp_page(cp.clone())?;
                    ctx.replace_page(page);
                    Ok(())
                }
            })
            .with_hint(cp_lock.current_problem()),
            button("e", "Edit", {
                let cp = cp.clone();
                move |mut ctx: Context| {
                    ctx.leave_ui()?;
                    {
                        let mut cp = cp.lock().unwrap();
                        if cp.current_contest().is_none() {
                            cp.select_contest_with_fzf()?;
                        }
                        if cp.current_problem().is_none() {
                            cp.select_problem_with_fzf()?;
                        }
                        let file = cp.current_problem_path()?.join("main.cpp");
                        let _ = File::create(&file)?;
                        ctx.run_command_new_term(Command::new("hx").arg(file))?;
                    }
                    let page = cp_page(cp.clone())?;
                    ctx.replace_page(page);
                    Ok(())
                }
            }),
            button("tt", "Run Tests", {
                let cp = cp.clone();
                move |mut ctx: Context| {
                    ctx.leave_ui()?;
                    ctx.show_cmd()?;
                    {
                        let mut cp = cp.lock().unwrap();
                        if cp.current_contest().is_none() {
                            cp.select_contest_with_fzf()?;
                        }
                        if cp.current_problem().is_none() {
                            cp.select_problem_with_fzf()?;
                        }
                        cp.run_tests()?;
                        let (input, output) = cp.new_test_case()?;
                        ctx.run_command_new_term(
                            Command::new("hx").arg("--vsplit").arg(input).arg(output),
                        )?;
                    }
                    let page = cp_page(cp.clone())?;
                    ctx.replace_page(page);
                    Ok(())
                }
            }),
            button("ta", "Add Test case", {
                let cp = cp.clone();
                move |mut ctx: Context| {
                    ctx.leave_ui()?;
                    {
                        let mut cp = cp.lock().unwrap();
                        if cp.current_contest().is_none() {
                            cp.select_contest_with_fzf()?;
                        }
                        if cp.current_problem().is_none() {
                            cp.select_problem_with_fzf()?;
                        }
                        let (input, output) = cp.new_test_case()?;
                        ctx.run_command_new_term(
                            Command::new("hx").arg("--vsplit").arg(input).arg(output),
                        )?;
                    }
                    let page = cp_page(cp.clone())?;
                    ctx.replace_page(page);
                    Ok(())
                }
            }),
        ],
    )]))
}
