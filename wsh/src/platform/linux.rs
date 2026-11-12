use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::string::String;
use std::vec::Vec;

use crate::parser::{Pipeline, Redirect, SimpleCommand};

pub fn get_cwd() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| String::from("/"))
}

pub fn set_cwd(path: &str) -> Result<(), String> {
    std::env::set_current_dir(path).map_err(|e| e.to_string())
}

pub fn get_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

pub fn set_env(key: &str, val: &str) {
    unsafe {
        std::env::set_var(key, val);
    }
}

pub fn unset_env(key: &str) {
    unsafe {
        std::env::remove_var(key);
    }
}

pub fn get_all_env() -> Vec<(String, String)> {
    std::env::vars().collect()
}

pub fn is_tty() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) == 1 }
}

pub struct RawModeGuard {
    orig_termios: libc::termios,
    active: bool,
}

impl RawModeGuard {
    pub fn enter() -> Option<Self> {
        if !is_tty() {
            return None;
        }

        unsafe {
            let mut orig: libc::termios = core::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut orig) != 0 {
                return None;
            }

            let mut raw = orig;
            // Отключаем эхо, канонический режим, постобработку сигналов в raw-режиме редактора
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL);
            raw.c_cc[libc::VMIN] = 1;
            raw.c_cc[libc::VTIME] = 0;

            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &raw) != 0 {
                return None;
            }

            Some(Self {
                orig_termios: orig,
                active: true,
            })
        }
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &self.orig_termios);
            }
        }
    }
}

pub fn read_stdin_chunk(buf: &mut [u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }
    match io::stdin().read(buf) {
        Ok(n) => n,
        Err(_) => 0,
    }
}

pub fn read_stdin_byte() -> Option<u8> {
    let mut buf = [0u8; 1];
    let n = io::stdin().read(&mut buf).ok()?;
    if n == 1 {
        Some(buf[0])
    } else {
        None
    }
}

/// Выполняет внешний конвейер команд с поддержкой каналов (|) и перенаправлений (>, >>, <)
pub fn execute_pipeline(pipeline: &Pipeline) -> i32 {
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return 0;
    }

    if num_cmds == 1 {
        return execute_single_command(&pipeline.commands[0], Stdio::inherit(), Stdio::inherit());
    }

    let mut prev_stdout: Option<std::process::ChildStdout> = None;
    let mut children = Vec::new();

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == num_cmds - 1;

        let stdin = if is_first {
            Stdio::inherit()
        } else if let Some(pipe) = prev_stdout.take() {
            Stdio::from(pipe)
        } else {
            Stdio::null()
        };

        let stdout = if is_last {
            Stdio::inherit()
        } else {
            Stdio::piped()
        };

        let mut process = Command::new(&cmd.name);
        process.args(&cmd.args);
        process.stdin(stdin);
        process.stdout(stdout);

        // Применяем перенаправления ввода-вывода
        for red in &cmd.redirects {
            match red {
                Redirect::Output(path) => {
                    if let Ok(file) = File::create(path) {
                        process.stdout(Stdio::from(file));
                    }
                }
                Redirect::AppendOutput(path) => {
                    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
                        process.stdout(Stdio::from(file));
                    }
                }
                Redirect::Input(path) => {
                    if let Ok(file) = File::open(path) {
                        process.stdin(Stdio::from(file));
                    }
                }
            }
        }

        match process.spawn() {
            Ok(mut child) => {
                if !is_last {
                    prev_stdout = child.stdout.take();
                }
                children.push(child);
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    eprintln!("wsh: command not found: {}", cmd.name);
                    return 127;
                } else {
                    eprintln!("wsh: error executing {}: {}", cmd.name, e);
                    return 1;
                }
            }
        }
    }

    let mut last_status = 0;
    for mut child in children {
        match child.wait() {
            Ok(status) => {
                last_status = status.code().unwrap_or(1);
            }
            Err(e) => {
                eprintln!("wsh: error waiting for child process: {}", e);
                last_status = 1;
            }
        }
    }

    last_status
}

fn execute_single_command(cmd: &SimpleCommand, default_in: Stdio, default_out: Stdio) -> i32 {
    let mut process = Command::new(&cmd.name);
    process.args(&cmd.args);
    process.stdin(default_in);
    process.stdout(default_out);

    for red in &cmd.redirects {
        match red {
            Redirect::Output(path) => match File::create(path) {
                Ok(file) => {
                    process.stdout(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("wsh: cannot create file {}: {}", path, e);
                    return 1;
                }
            },
            Redirect::AppendOutput(path) => match OpenOptions::new().create(true).append(true).open(path) {
                Ok(file) => {
                    process.stdout(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("wsh: cannot open file {}: {}", path, e);
                    return 1;
                }
            },
            Redirect::Input(path) => match File::open(path) {
                Ok(file) => {
                    process.stdin(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("wsh: cannot open file {}: {}", path, e);
                    return 1;
                }
            },
        }
    }

    match process.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => status.code().unwrap_or(0),
            Err(e) => {
                eprintln!("wsh: failed to wait on child: {}", e);
                1
            }
        },
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                eprintln!("wsh: command not found: {}", cmd.name);
                127
            } else {
                eprintln!("wsh: execution error: {}", e);
                1
            }
        }
    }
}

pub fn read_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub fn append_file_line(path: &str, line: &str) -> bool {
    use std::io::Write;
    let mut file = match OpenOptions::new().create(true).append(true).open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    if writeln!(file, "{}", line).is_err() {
        return false;
    }
    true
}

pub fn write_file(path: &str, content: &str) -> bool {
    std::fs::write(path, content).is_ok()
}

pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

pub fn is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

pub fn file_size(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn resolve_in_path(cmd_name: &str) -> String {
    if cmd_name.contains('/') {
        return String::from(cmd_name);
    }

    let path_val = get_env("PATH").unwrap_or_else(|| String::from("/bin:/usr/bin"));
    for dir in path_val.split(':') {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            continue;
        }
        let p = std::path::Path::new(trimmed).join(cmd_name);
        if p.is_file() {
            return p.to_string_lossy().into_owned();
        }
    }

    String::from(cmd_name)
}
