use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use eyre::Result;

use crate::FD_PID;
pub static USING_STDOUT: AtomicBool = AtomicBool::new(false);

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
        }
    }

    pub fn file_name(&mut self, name: &str) {
        self.args.push(name.to_string());
    }

    pub fn run<F: Fn(&str)>(&mut self, cb: F) -> Result<()> {
        let cmd = Command::new("fd")
            .args(&self.args)
            .stdout(Stdio::piped())
            .spawn()?;
        {
            USING_STDOUT.store(true, Ordering::Relaxed);
            FD_PID.store(cmd.id() as i32, Ordering::SeqCst);
            let stdout = Arc::new(Mutex::new(cmd.stdout));
            let stdout_clone = stdout.clone();

            thread::spawn(move || loop {
                if FD_PID.load(Ordering::SeqCst) == -1 {
                    let mut stdout = stdout_clone.lock().unwrap();
                    USING_STDOUT.store(false, Ordering::Relaxed);
                    drop(stdout.take());
                    break;
                }
            });

            {
                let mut stdout = stdout.lock().unwrap();
                let stdout_reader = BufReader::new(stdout.as_mut().unwrap());
                let stdout_lines = stdout_reader.lines();

                for i in stdout_lines.flatten() {
                    if USING_STDOUT.load(Ordering::Relaxed) {
                        cb(&i);
                    } else {
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }
}
