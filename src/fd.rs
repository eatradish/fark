use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio, Child},
};

use eyre::Result;

pub struct FdCommand {
    args: Vec<String>,
}

impl FdCommand {
    pub fn new() -> Self {
        Self { args: vec![] }
    }

    pub fn set_path(&mut self, path: &str) {
        self.args.push("--search-path".to_string());
        self.args.push(path.to_string());
    }

    pub fn glob(&mut self, b: bool) {
        if !self.args.contains(&"--glob".to_string()) && b {
            self.args.push("--glob".to_string());
            return;
        }

        if self.args.contains(&"--glob".to_string()) && !b {
            let index = self.args.iter().position(|x| x == "--glob").unwrap();
            self.args.remove(index);
            return;
        }
    }

    pub fn file_name(&mut self, name: &str) {
        self.args.push(name.to_string());
    }

    pub fn run<F>(&mut self, cb: F) -> Result<Child>
    where
        F: Fn(&str),
    {
        let mut cmd = Command::new("fd")
            .args(&self.args)
            .stdout(Stdio::piped())
            .spawn()?;
        {
            let stdout = cmd
                .stdout
                .take()
                .unwrap();

            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            for i in stdout_lines.flatten() {
                cb(&i);
            }
        }

        Ok(cmd)
    }
}
