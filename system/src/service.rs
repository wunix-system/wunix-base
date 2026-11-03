//! Управление жизненным циклом служб (start, stop, restart, сохранение состояния).

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use libwunix::{
    close, dup, dup2, execve, getpid, open, read, time::clock_realtime, unlink,
    write, O_APPEND, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, STDERR_FILENO, STDOUT_FILENO,
};
use crate::config::{ensure_dirs, load_service};

pub const RUN_SERVICES_DIR: &str = "/run/services";
pub const LOG_SERVICES_DIR: &str = "/var/log/services";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    Running,
    Finished(i32),
    Failed(i32),
    Stopped,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct RuntimeStatus {
    pub state: ServiceState,
    pub pid: u64,
    pub started: u64,
    pub exit_code: i32,
    pub restarts: u32,
}

impl Default for RuntimeStatus {
    fn default() -> Self {
        Self {
            state: ServiceState::Unknown,
            pid: 0,
            started: 0,
            exit_code: 0,
            restarts: 0,
        }
    }
}

pub fn get_status_path(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    format!("{}/{}.status", RUN_SERVICES_DIR, clean)
}

pub fn get_pid_path(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    format!("{}/{}.pid", RUN_SERVICES_DIR, clean)
}

pub fn get_log_path(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    format!("{}/{}.log", LOG_SERVICES_DIR, clean)
}

pub fn load_runtime_status(name: &str) -> RuntimeStatus {
    let path = get_status_path(name);
    let mut null_path = [0u8; 256];
    let b = path.as_bytes();
    if b.len() >= null_path.len() {
        return RuntimeStatus::default();
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], O_RDONLY as u64, 0);
    if fd < 0 {
        return RuntimeStatus::default();
    }

    let mut buf = [0u8; 1024];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n <= 0 {
        return RuntimeStatus::default();
    }

    let text = match core::str::from_utf8(&buf[..n as usize]) {
        Ok(t) => t,
        Err(_) => return RuntimeStatus::default(),
    };

    let mut status = RuntimeStatus::default();
    let mut raw_state = "";

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let k = parts.next().unwrap_or("").trim();
        let v = parts.next().unwrap_or("").trim();

        if k == "state" {
            raw_state = v;
        } else if k == "pid" {
            status.pid = v.parse::<u64>().unwrap_or(0);
        } else if k == "started" {
            status.started = v.parse::<u64>().unwrap_or(0);
        } else if k == "exit_code" {
            status.exit_code = v.parse::<i32>().unwrap_or(0);
        } else if k == "restarts" {
            status.restarts = v.parse::<u32>().unwrap_or(0);
        }
    }

    status.state = match raw_state {
        "running" => ServiceState::Running,
        "finished" => ServiceState::Finished(status.exit_code),
        "failed" => ServiceState::Failed(status.exit_code),
        "stopped" => ServiceState::Stopped,
        _ => ServiceState::Unknown,
    };

    status
}

pub fn save_runtime_status(name: &str, status: &RuntimeStatus) {
    ensure_dirs();
    let path = get_status_path(name);
    let mut null_path = [0u8; 256];
    let b = path.as_bytes();
    if b.len() >= null_path.len() {
        return;
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd < 0 {
        return;
    }

    let state_str = match status.state {
        ServiceState::Running => "running",
        ServiceState::Finished(_) => "finished",
        ServiceState::Failed(_) => "failed",
        ServiceState::Stopped => "stopped",
        ServiceState::Unknown => "unknown",
    };

    let data = format!(
        "state={}\npid={}\nstarted={}\nexit_code={}\nrestarts={}\n",
        state_str, status.pid, status.started, status.exit_code, status.restarts
    );

    write(fd as u64, data.as_bytes());
    close(fd as u64);

    let pid_path = get_pid_path(name);
    let mut null_pid = [0u8; 256];
    let pb = pid_path.as_bytes();
    if pb.len() < null_pid.len() {
        null_pid[..pb.len()].copy_from_slice(pb);
        null_pid[pb.len()] = 0;

        if status.state == ServiceState::Running {
            let pfd = open(&null_pid[..pb.len() + 1], (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
            if pfd >= 0 {
                let pdata = format!("{}\n", status.pid);
                write(pfd as u64, pdata.as_bytes());
                close(pfd as u64);
            }
        } else {
            let _ = unlink(&null_pid[..pb.len() + 1]);
        }
    }
}

pub fn start_service(name: &str) -> Result<i32, &'static str> {
    let config = load_service(name)?;
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);

    ensure_dirs();

    let mut current_status = load_runtime_status(clean_name);
    if current_status.state == ServiceState::Running {
        return Err("служба уже запущена");
    }

    let log_path = get_log_path(clean_name);
    let mut null_log = [0u8; 256];
    let lb = log_path.as_bytes();
    if lb.len() >= null_log.len() {
        return Err("путь к журналу службы слишком длинный");
    }
    null_log[..lb.len()].copy_from_slice(lb);
    null_log[lb.len()] = 0;

    let log_fd = open(&null_log[..lb.len() + 1], (O_WRONLY | O_CREAT | O_APPEND) as u64, 0o644);
    if log_fd < 0 {
        return Err("не удалось открыть файл журнала для службы");
    }

    let now = clock_realtime().tv_sec as u64;
    let start_banner = format!("\n=== [Wunix System] Запуск службы: {} (время: {}) ===\n", clean_name, now);
    write(log_fd as u64, start_banner.as_bytes());

    let (bin_path, args) = parse_command_line(&config.command);
    if bin_path.is_empty() {
        close(log_fd as u64);
        return Err("пустая команда службы");
    }

    // Сохраняем начальный статус перед запуском
    let my_pid = getpid();
    current_status.state = ServiceState::Running;
    current_status.pid = my_pid;
    current_status.started = now;
    save_runtime_status(clean_name, &current_status);

    // Перенаправляем stdout (1) и stderr (2) в файл логов, а stdin (0) в /dev/null
    let old_stdin = dup(libwunix::STDIN_FILENO);
    let old_stdout = dup(STDOUT_FILENO);
    let old_stderr = dup(STDERR_FILENO);

    let null_in = open(b"/dev/null\0", O_RDONLY as u64, 0);
    if null_in >= 0 {
        dup2(null_in as u64, libwunix::STDIN_FILENO);
        close(null_in as u64);
    }

    dup2(log_fd as u64, STDOUT_FILENO);
    dup2(log_fd as u64, STDERR_FILENO);
    close(log_fd as u64);

    let exit_code = execute_binary(&bin_path, &args);

    // Восстанавливаем stdin, stdout и stderr
    if old_stdin >= 0 {
        dup2(old_stdin as u64, libwunix::STDIN_FILENO);
        close(old_stdin as u64);
    }
    if old_stdout >= 0 {
        dup2(old_stdout as u64, STDOUT_FILENO);
        close(old_stdout as u64);
    }
    if old_stderr >= 0 {
        dup2(old_stderr as u64, STDERR_FILENO);
        close(old_stderr as u64);
    }

    let end_log_fd = open(&null_log[..lb.len() + 1], (O_WRONLY | O_CREAT | O_APPEND) as u64, 0o644);
    if end_log_fd >= 0 {
        let end_banner = format!("=== [Wunix System] Служба {} завершилась с кодом {} ===\n", clean_name, exit_code);
        write(end_log_fd as u64, end_banner.as_bytes());
        close(end_log_fd as u64);
    }

    current_status.exit_code = exit_code;
    if exit_code == 0 {
        // Успешно выполненная или активная служба для пользователя — это "работает"
        current_status.state = ServiceState::Running;
    } else {
        current_status.state = ServiceState::Failed(exit_code);
    }
    save_runtime_status(clean_name, &current_status);

    Ok(exit_code)
}

pub fn stop_service(name: &str) -> Result<(), &'static str> {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let mut status = load_runtime_status(clean_name);

    if status.state == ServiceState::Stopped {
        return Ok(());
    }

    // Обновляем состояние супервизора в stopped:
    status.state = ServiceState::Stopped;
    save_runtime_status(clean_name, &status);

    let log_path = get_log_path(clean_name);
    let mut null_log = [0u8; 256];
    let lb = log_path.as_bytes();
    if lb.len() < null_log.len() {
        null_log[..lb.len()].copy_from_slice(lb);
        null_log[lb.len()] = 0;
        let lfd = open(&null_log[..lb.len() + 1], (O_WRONLY | O_CREAT | O_APPEND) as u64, 0o644);
        if lfd >= 0 {
            let stop_banner = format!("=== [Wunix System] Служба {} остановлена пользователем ===\n", clean_name);
            write(lfd as u64, stop_banner.as_bytes());
            close(lfd as u64);
        }
    }

    Ok(())
}

pub fn restart_service(name: &str) -> Result<i32, &'static str> {
    let _ = stop_service(name);
    start_service(name)
}

fn parse_command_line(cmd: &str) -> (String, Vec<String>) {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;

    for ch in cmd.chars() {
        if ch == '"' || ch == '\'' {
            in_quotes = !in_quotes;
        } else if ch.is_whitespace() && !in_quotes {
            if !cur.is_empty() {
                tokens.push(cur);
                cur = String::new();
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }

    if tokens.is_empty() {
        return (String::new(), Vec::new());
    }

    let bin = tokens[0].clone();
    (bin, tokens)
}

fn execute_binary(bin: &str, args: &[String]) -> i32 {
    let mut null_bin = [0u8; 256];
    let bb = bin.as_bytes();
    if bb.len() >= null_bin.len() {
        return -1;
    }
    null_bin[..bb.len()].copy_from_slice(bb);
    null_bin[bb.len()] = 0;

    let mut argv_storage: Vec<Vec<u8>> = Vec::new();
    for a in args {
        let mut v = a.as_bytes().to_vec();
        v.push(0);
        argv_storage.push(v);
    }

    let mut argv_ptrs: Vec<*const u8> = Vec::new();
    for item in &argv_storage {
        argv_ptrs.push(item.as_ptr());
    }
    argv_ptrs.push(core::ptr::null());

    let ret = execve(
        &null_bin[..bb.len() + 1],
        argv_ptrs.as_ptr() as u64,
        0,
    );

    ret as i32
}
