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

pub fn is_fd_tty(fd: u64) -> bool {
    unsafe { libc::isatty(fd as i32) == 1 }
}

pub fn get_terminal_size() -> (usize, usize) {
    unsafe {
        let mut ws: libc::winsize = core::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0
            && ws.ws_col > 0
            && ws.ws_row > 0
        {
            (ws.ws_col as usize, ws.ws_row as usize)
        } else {
            (80, 24)
        }
    }
}

pub fn has_pending_stdin_byte_briefly() -> bool {
    if !is_tty() {
        return true;
    }
    let mut pfd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };
    let rc = unsafe { libc::poll(&mut pfd, 1, 20) };
    rc > 0 && (pfd.revents & libc::POLLIN) != 0
}

pub fn run_capture_with_env(args: &[String], extra_env: &[(&str, &str)]) -> Option<String> {
    let prog = args.first()?;
    let mut cmd = Command::new(prog);
    cmd.args(&args[1..])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let output = cmd.output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    }
}

pub fn can_read(path: &str) -> bool {
    if let Ok(cstr) = std::ffi::CString::new(path) {
        unsafe { libc::access(cstr.as_ptr(), libc::R_OK) == 0 }
    } else {
        false
    }
}

pub fn can_write(path: &str) -> bool {
    if let Ok(cstr) = std::ffi::CString::new(path) {
        unsafe { libc::access(cstr.as_ptr(), libc::W_OK) == 0 }
    } else {
        false
    }
}

pub fn capture_stdout<F: FnOnce() -> i32>(f: F) -> (i32, String) {
    static SUB_SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
    let seq = SUB_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp_path = std::env::temp_dir().join(format!("wsh_sub_{}_{}", std::process::id(), seq));
    let Ok(file) = File::create(&tmp_path) else {
        return (f(), String::new());
    };
    use std::os::unix::io::AsRawFd;
    let out_fd = file.as_raw_fd();
    unsafe {
        let saved = libc::dup(1);
        libc::dup2(out_fd, 1);
        drop(file);
        let code = f();
        if saved >= 0 {
            libc::dup2(saved, 1);
            libc::close(saved);
        }
        let content = std::fs::read_to_string(&tmp_path).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_path);
        let trimmed = content.trim_end_matches('\n').trim_end_matches('\r');
        (code, String::from(trimmed))
    }
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
    execute_pipeline_with_runner(pipeline, false, |_| None)
}

pub fn execute_pipeline_with_runner<F>(
    pipeline: &Pipeline,
    pipefail: bool,
    mut run_builtin: F,
) -> i32
where
    F: FnMut(&SimpleCommand) -> Option<i32>,
{
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return 0;
    }

    if num_cmds == 1 {
        if let Some(code) = run_builtin(&pipeline.commands[0]) {
            return code;
        }
        return execute_single_command(&pipeline.commands[0], Stdio::inherit(), Stdio::inherit());
    }

    // Если среди стадий есть встроенные команды, выполняем их последовательно с перенаправлением
    let mut last_status = 0;
    let mut pipefail_status = 0;
    let mut prev_bytes: Option<Vec<u8>> = None;

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let is_last = i == num_cmds - 1;
        if let Some(code) = run_builtin(cmd) {
            if !is_last {
                prev_bytes = Some(Vec::new());
            }
            last_status = code;
            if code != 0 {
                pipefail_status = code;
            }
            continue;
        }

        let mut process = Command::new(&cmd.name);
        process.args(&cmd.args);
        if prev_bytes.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::inherit());
        }
        if is_last {
            process.stdout(Stdio::inherit());
        } else {
            process.stdout(Stdio::piped());
        }
        for red in &cmd.redirects {
            match red {
                Redirect::Output { fd: 1, path, append } => {
                    let f = if *append {
                        OpenOptions::new().create(true).append(true).open(path)
                    } else {
                        File::create(path)
                    };
                    if let Ok(file) = f {
                        process.stdout(Stdio::from(file));
                    }
                }
                Redirect::Output { fd: 2, path, append } => {
                    let f = if *append {
                        OpenOptions::new().create(true).append(true).open(path)
                    } else {
                        File::create(path)
                    };
                    if let Ok(file) = f {
                        process.stderr(Stdio::from(file));
                    }
                }
                Redirect::Input { fd: 0, path } => {
                    if let Ok(file) = File::open(path) {
                        process.stdin(Stdio::from(file));
                    }
                }
                _ => {}
            }
        }

        match process.spawn() {
            Ok(mut child) => {
                if let Some(input) = prev_bytes.take() {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        let _ = stdin.write_all(&input);
                    }
                }
                match child.wait_with_output() {
                    Ok(out) => {
                        if !is_last {
                            prev_bytes = Some(out.stdout);
                        }
                        last_status = out.status.code().unwrap_or(1);
                    }
                    Err(e) => {
                        eprintln!("wsh: error waiting for child process: {}", e);
                        last_status = 1;
                    }
                }
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    eprintln!("wsh: command not found: {}", cmd.name);
                    last_status = 127;
                } else {
                    eprintln!("wsh: error executing {}: {}", cmd.name, e);
                    last_status = 1;
                }
            }
        }
        if last_status != 0 {
            pipefail_status = last_status;
        }
    }

    if pipefail && pipefail_status != 0 {
        pipefail_status
    } else {
        last_status
    }
}

fn execute_single_command(cmd: &SimpleCommand, default_in: Stdio, default_out: Stdio) -> i32 {
    let mut process = Command::new(&cmd.name);
    process.args(&cmd.args);
    process.stdin(default_in);
    process.stdout(default_out);

    for red in &cmd.redirects {
        match red {
            Redirect::Output { fd: 1, path, append } => {
                let f = if *append {
                    OpenOptions::new().create(true).append(true).open(path)
                } else {
                    File::create(path)
                };
                match f {
                    Ok(file) => {
                        process.stdout(Stdio::from(file));
                    }
                    Err(e) => {
                        eprintln!("wsh: cannot open file {}: {}", path, e);
                        return 1;
                    }
                }
            }
            Redirect::Output { fd: 2, path, append } => {
                let f = if *append {
                    OpenOptions::new().create(true).append(true).open(path)
                } else {
                    File::create(path)
                };
                match f {
                    Ok(file) => {
                        process.stderr(Stdio::from(file));
                    }
                    Err(e) => {
                        eprintln!("wsh: cannot open file {}: {}", path, e);
                        return 1;
                    }
                }
            }
            Redirect::Input { fd: 0, path } => match File::open(path) {
                Ok(file) => {
                    process.stdin(Stdio::from(file));
                }
                Err(e) => {
                    eprintln!("wsh: cannot open file {}: {}", path, e);
                    return 1;
                }
            },
            _ => {}
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
