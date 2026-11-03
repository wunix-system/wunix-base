extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use crate::parser::Pipeline;
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
    String::from("/")
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

pub struct RawModeGuard;

impl RawModeGuard {
    pub fn enter() -> Option<Self> {
        Some(Self)
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

pub fn execute_pipeline(pipeline: &Pipeline) -> i32 {
    if pipeline.commands.is_empty() {
        return 0;
    }

    let cmd = &pipeline.commands[0];
    let mut null_term_path = [0u8; PATH_BUFFER_CAPACITY];
    let path_bytes = cmd.name.as_bytes();
    if path_bytes.len() >= null_term_path.len() {
        libwunix::println!("wsh: executable path too long");
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
}
