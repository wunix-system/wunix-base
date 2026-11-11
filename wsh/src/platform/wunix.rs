extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::parser::{Pipeline, Redirect, SimpleCommand};
use libwunix::STDIN_FILENO;

const PATH_BUFFER_CAPACITY: usize = 256;

pub fn get_cwd() -> String {
    let mut buf = [0u8; PATH_BUFFER_CAPACITY];
    let ret = libwunix::getcwd(&mut buf);
    if ret > 0 {
        let len = buf.iter().position(|&b| b == 0).unwrap_or(ret as usize);
        if let Ok(s) = core::str::from_utf8(&buf[..len]) {
            return String::from(s);
        }
    }
    let euid = libwunix::geteuid();
    if euid == 0 {
        String::from("/root")
    } else {
        String::from("/home")
    }
}

pub fn set_cwd(path: &str) -> Result<(), String> {
    let mut null_terminated = [0u8; PATH_BUFFER_CAPACITY];
    let bytes = path.as_bytes();
    if bytes.len() >= null_terminated.len() {
        return Err(String::from("path too long"));
    }
    null_terminated[..bytes.len()].copy_from_slice(bytes);
    null_terminated[bytes.len()] = 0;

    let ret = libwunix::chdir(&null_terminated);
    if ret >= 0 {
        Ok(())
    } else {
        Err(String::from("cannot change directory"))
    }
}

struct SimpleMutex<T> {
    lock: core::sync::atomic::AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SimpleMutex<T> {}

impl<T> SimpleMutex<T> {
    const fn new(data: T) -> Self {
        Self {
            lock: core::sync::atomic::AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }

    fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.lock.swap(true, core::sync::atomic::Ordering::Acquire) {
            core::hint::spin_loop();
        }
        let res = f(unsafe { &mut *self.data.get() });
        self.lock.store(false, core::sync::atomic::Ordering::Release);
        res
    }
}

static ENV_VARS: SimpleMutex<Option<Vec<(String, String)>>> = SimpleMutex::new(None);

fn init_env_if_needed(vars: &mut Option<Vec<(String, String)>>) -> &mut Vec<(String, String)> {
    if vars.is_none() {
        let mut v = Vec::new();
        let euid = libwunix::geteuid();
        let (uname, home) = if let Ok(Some(pwd)) = libwunix::get_passwd_by_uid(euid) {
            (String::from(pwd.username()), String::from(pwd.home()))
        } else if euid == 0 {
            (String::from("root"), String::from("/root"))
        } else {
            (String::from("user"), String::from("/home"))
        };
        v.push((String::from("USER"), uname));
        v.push((String::from("HOME"), home));
        v.push((String::from("HOSTNAME"), String::from("wunix")));
        v.push((String::from("SHELL"), String::from("/bin/wsh")));
        v.push((String::from("PATH"), String::from("/bin:/usr/bin")));
        v.push((String::from("TERM"), String::from("linux")));
        v.push((String::from("BASH_VERSION"), String::from("5.2.0(1)-release")));

        // Загрузка глобальных переменных окружения из /etc/environment
        if let Some(content) = read_file("/etc/environment") {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((k, val)) = trimmed.split_once('=') {
                    let k = k.trim();
                    let clean_val = val.trim().trim_matches('"').trim_matches('\'');
                    if !k.is_empty() {
                        if let Some((_, existing)) = v.iter_mut().find(|(name, _)| name == k) {
                            *existing = String::from(clean_val);
                        } else {
                            v.push((String::from(k), String::from(clean_val)));
                        }
                    }
                }
            }
        }

        *vars = Some(v);
    }
    vars.as_mut().unwrap()
}

pub fn get_env(key: &str) -> Option<String> {
    ENV_VARS.lock(|opt| {
        let vars = init_env_if_needed(opt);
        for (k, v) in vars.iter() {
            if k == key {
                return Some(v.clone());
            }
        }
        None
    })
}

pub fn set_env(key: &str, val: &str) {
    ENV_VARS.lock(|opt| {
        let vars = init_env_if_needed(opt);
        for (k, v) in vars.iter_mut() {
            if k == key {
                *v = String::from(val);
                return;
            }
        }
        vars.push((String::from(key), String::from(val)));
    });
}

pub fn unset_env(key: &str) {
    ENV_VARS.lock(|opt| {
        let vars = init_env_if_needed(opt);
        vars.retain(|(k, _)| k != key);
    });
}

pub fn get_all_env() -> Vec<(String, String)> {
    ENV_VARS.lock(|opt| {
        let vars = init_env_if_needed(opt);
        vars.clone()
    })
}

pub fn is_tty() -> bool {
    true
}

pub struct RawModeGuard {
    orig_termios: libwunix::Termios,
    active: bool,
}

impl RawModeGuard {
    pub fn enter() -> Option<Self> {
        let mut orig = libwunix::Termios::default();
        if libwunix::tcgetattr(STDIN_FILENO, &mut orig) != 0 {
            return None;
        }

        let mut raw = orig;
        // Отключаем эхо, канонический режим и генерацию сигналов ядра (ISIG),
        // чтобы wsh сам перехватывал Ctrl+C/Ctrl+D, управлял историей и мгновенно вставлял буфер.
        raw.c_lflag &= !(libwunix::ECHO | libwunix::ICANON | libwunix::ISIG | libwunix::IEXTEN);
        raw.c_iflag &= !(libwunix::IXON | libwunix::ICRNL);
        raw.c_cc[libwunix::VMIN] = 1;
        raw.c_cc[libwunix::VTIME] = 0;

        if libwunix::tcsetattr(STDIN_FILENO, 0, &raw) != 0 {
            return None;
        }

        Some(Self {
            orig_termios: orig,
            active: true,
        })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            libwunix::tcsetattr(STDIN_FILENO, 0, &self.orig_termios);
        }
    }
}

pub fn read_stdin_chunk(buf: &mut [u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }
    let n = libwunix::read(STDIN_FILENO, buf);
    if n > 0 {
        n as usize
    } else {
        0
    }
}

pub fn read_stdin_byte() -> Option<u8> {
    let mut b = [0u8; 1];
    let n = libwunix::read(STDIN_FILENO, &mut b);
    if n == 1 {
        Some(b[0])
    } else {
        None
    }
}

fn format_u64<'a>(mut val: u64, buf: &'a mut [u8; 20]) -> &'a str {
    if val == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap_or("0");
    }
    let mut i = buf.len();
    while val > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("0")
}

pub fn execute_pipeline(pipeline: &Pipeline) -> i32 {
    let num_cmds = pipeline.commands.len();
    if num_cmds == 0 {
        return 0;
    }

    if num_cmds == 1 {
        return execute_command_with_redirects(&pipeline.commands[0], None, None);
    }

    // Для конвейера из нескольких команд (cmd1 | cmd2 | ...) используем временные файлы в ramfs (/tmp)
    static PIPE_SEQ: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1);
    let mut prev_temp_file: Option<String> = None;
    let mut prev_read_fd: Option<i64> = None;
    let mut last_status = 0;

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == num_cmds - 1;

        let in_fd = if is_first {
            None
        } else {
            prev_read_fd.take()
        };

        let (out_fd, current_temp_file) = if is_last {
            (None, None)
        } else {
            let seq = PIPE_SEQ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            let mut path_str = String::from("/tmp/.wsh_p_");
            let mut num_buf = [0u8; 20];
            let num_s = format_u64(seq as u64, &mut num_buf);
            path_str.push_str(num_s);

            let mut null_term = [0u8; PATH_BUFFER_CAPACITY];
            let b = path_str.as_bytes();
            if b.len() < null_term.len() {
                null_term[..b.len()].copy_from_slice(b);
                null_term[b.len()] = 0;
                let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_TRUNC) as u64;
                let fd = libwunix::open(&null_term[..b.len() + 1], flags, 0o600);
                if fd >= 0 {
                    (Some(fd), Some(path_str))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        };

        last_status = execute_command_with_redirects(cmd, in_fd, out_fd);

        // Закрываем дескриптор чтения предыдущего шага и удаляем его временный файл
        if let Some(fd) = in_fd {
            let _ = libwunix::close(fd as u64);
        }
        if let Some(old_path) = prev_temp_file.take() {
            let mut null_term = [0u8; PATH_BUFFER_CAPACITY];
            let b = old_path.as_bytes();
            if b.len() < null_term.len() {
                null_term[..b.len()].copy_from_slice(b);
                null_term[b.len()] = 0;
                let _ = libwunix::unlink(&null_term[..b.len() + 1]);
            }
        }

        // Закрываем дескриптор записи текущего шага
        if let Some(fd) = out_fd {
            let _ = libwunix::close(fd as u64);
        }

        // Открываем записанный файл для чтения следующей командой конвейера
        if let Some(ref path_str) = current_temp_file {
            let mut null_term = [0u8; PATH_BUFFER_CAPACITY];
            let b = path_str.as_bytes();
            if b.len() < null_term.len() {
                null_term[..b.len()].copy_from_slice(b);
                null_term[b.len()] = 0;
                let fd = libwunix::open(&null_term[..b.len() + 1], libwunix::O_RDONLY as u64, 0);
                if fd >= 0 {
                    prev_read_fd = Some(fd);
                }
            }
        }
        prev_temp_file = current_temp_file;
    }

    // Удаляем последний оставшийся временный файл
    if let Some(old_path) = prev_temp_file {
        let mut null_term = [0u8; PATH_BUFFER_CAPACITY];
        let b = old_path.as_bytes();
        if b.len() < null_term.len() {
            null_term[..b.len()].copy_from_slice(b);
            null_term[b.len()] = 0;
            let _ = libwunix::unlink(&null_term[..b.len() + 1]);
        }
    }
    if let Some(fd) = prev_read_fd {
        let _ = libwunix::close(fd as u64);
    }

    last_status
}

fn execute_command_with_redirects(
    cmd: &SimpleCommand,
    stdin_fd: Option<i64>,
    stdout_fd: Option<i64>,
) -> i32 {
    let mut null_term_path = [0u8; PATH_BUFFER_CAPACITY];
    let path_bytes = cmd.name.as_bytes();
    if path_bytes.len() >= null_term_path.len() {
        libwunix::write(libwunix::STDERR_FILENO, b"wsh: executable path too long\n");
        return 1;
    }
    null_term_path[..path_bytes.len()].copy_from_slice(path_bytes);
    null_term_path[path_bytes.len()] = 0;

    // Формируем нуль-терминированные строки для argv
    let mut argv_storage: Vec<Vec<u8>> = Vec::new();
    let mut name_vec = cmd.name.as_bytes().to_vec();
    name_vec.push(0);
    argv_storage.push(name_vec);

    for arg in &cmd.args {
        let mut arg_vec = arg.as_bytes().to_vec();
        arg_vec.push(0);
        argv_storage.push(arg_vec);
    }

    let mut argv_ptrs: Vec<*const u8> = Vec::new();
    for item in &argv_storage {
        argv_ptrs.push(item.as_ptr());
    }
    argv_ptrs.push(core::ptr::null());

    // Передаем переменные окружения шелла в дочерний процесс
    let current_env = get_all_env();
    let mut envp_storage: Vec<Vec<u8>> = Vec::new();
    for (k, v) in current_env {
        let mut entry = Vec::new();
        entry.extend_from_slice(k.as_bytes());
        entry.push(b'=');
        entry.extend_from_slice(v.as_bytes());
        entry.push(0);
        envp_storage.push(entry);
    }
    let mut envp_ptrs: Vec<*const u8> = Vec::new();
    for item in &envp_storage {
        envp_ptrs.push(item.as_ptr());
    }
    envp_ptrs.push(core::ptr::null());

    // Сохраняем исходные stdin/stdout, чтобы восстановить их после завершения дочернего процесса
    let saved_stdin = libwunix::dup(0);
    let saved_stdout = libwunix::dup(1);

    // Подключаем вход от предыдущего этапа конвейера
    if let Some(in_fd) = stdin_fd {
        let _ = libwunix::dup2(in_fd as u64, 0);
    }

    // Подключаем выход к следующему этапу конвейера
    if let Some(out_fd) = stdout_fd {
        let _ = libwunix::dup2(out_fd as u64, 1);
    }

    // Применяем явные перенаправления команды (>, >>, <)
    let mut redirect_error = false;
    for red in &cmd.redirects {
        match red {
            Redirect::Output(path) => {
                let mut p_buf = [0u8; PATH_BUFFER_CAPACITY];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_TRUNC) as u64;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], flags, 0o666);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 1);
                        let _ = libwunix::close(fd as u64);
                    } else {
                        libwunix::write(libwunix::STDERR_FILENO, b"wsh: cannot create file: ");
                        libwunix::write(libwunix::STDERR_FILENO, path.as_bytes());
                        libwunix::write(libwunix::STDERR_FILENO, b"\n");
                        redirect_error = true;
                        break;
                    }
                }
            }
            Redirect::AppendOutput(path) => {
                let mut p_buf = [0u8; PATH_BUFFER_CAPACITY];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_APPEND) as u64;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], flags, 0o666);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 1);
                        let _ = libwunix::close(fd as u64);
                    } else {
                        libwunix::write(libwunix::STDERR_FILENO, b"wsh: cannot open file: ");
                        libwunix::write(libwunix::STDERR_FILENO, path.as_bytes());
                        libwunix::write(libwunix::STDERR_FILENO, b"\n");
                        redirect_error = true;
                        break;
                    }
                }
            }
            Redirect::Input(path) => {
                let mut p_buf = [0u8; PATH_BUFFER_CAPACITY];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], libwunix::O_RDONLY as u64, 0);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 0);
                        let _ = libwunix::close(fd as u64);
                    } else {
                        libwunix::write(libwunix::STDERR_FILENO, b"wsh: cannot open file: ");
                        libwunix::write(libwunix::STDERR_FILENO, path.as_bytes());
                        libwunix::write(libwunix::STDERR_FILENO, b"\n");
                        redirect_error = true;
                        break;
                    }
                }
            }
        }
    }

    let status = if redirect_error {
        1
    } else {
        let ret = libwunix::execve(
            &null_term_path[..path_bytes.len() + 1],
            argv_ptrs.as_ptr() as u64,
            envp_ptrs.as_ptr() as u64,
        );

        if ret < 0 {
            let not_found_prefix = libwunix::t("команда не найдена: ", "command not found: ");
            libwunix::write(libwunix::STDERR_FILENO, not_found_prefix.as_bytes());
            libwunix::write(libwunix::STDERR_FILENO, cmd.name.as_bytes());
            libwunix::write(libwunix::STDERR_FILENO, b"\n");
            127
        } else {
            ret as i32
        }
    };

    // Восстанавливаем сохраненные стандартные дескрипторы шелла
    if saved_stdin >= 0 {
        let _ = libwunix::dup2(saved_stdin as u64, 0);
        let _ = libwunix::close(saved_stdin as u64);
    }
    if saved_stdout >= 0 {
        let _ = libwunix::dup2(saved_stdout as u64, 1);
        let _ = libwunix::close(saved_stdout as u64);
    }

    status
}

pub fn read_file(path: &str) -> Option<String> {
    let mut null_path = [0u8; PATH_BUFFER_CAPACITY];
    let bytes = path.as_bytes();
    if bytes.len() >= null_path.len() {
        return None;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let fd = libwunix::open(&null_path[..bytes.len() + 1], libwunix::O_RDONLY as u64, 0);
    if fd < 0 {
        return None;
    }

    let mut bytes_vec = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let n = libwunix::read(fd as u64, &mut buf);
        if n == 0 {
            break;
        }
        bytes_vec.extend_from_slice(&buf[..n as usize]);
    }
    let _ = libwunix::close(fd as u64);
    String::from_utf8(bytes_vec).ok()
}

pub fn append_file_line(path: &str, line: &str) -> bool {
    let mut null_path = [0u8; PATH_BUFFER_CAPACITY];
    let bytes = path.as_bytes();
    if bytes.len() >= null_path.len() {
        return false;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_APPEND) as u64;
    let fd = libwunix::open(&null_path[..bytes.len() + 1], flags, 0o600);
    if fd < 0 {
        return false;
    }

    let _ = libwunix::write(fd as u64, line.as_bytes());
    let _ = libwunix::write(fd as u64, b"
");
    let _ = libwunix::close(fd as u64);
    true
}

pub fn write_file(path: &str, content: &str) -> bool {
    let mut null_path = [0u8; PATH_BUFFER_CAPACITY];
    let bytes = path.as_bytes();
    if bytes.len() >= null_path.len() {
        return false;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_TRUNC) as u64;
    let fd = libwunix::open(&null_path[..bytes.len() + 1], flags, 0o600);
    if fd < 0 {
        return false;
    }

    let _ = libwunix::write(fd as u64, content.as_bytes());
    let _ = libwunix::close(fd as u64);
    true
}
