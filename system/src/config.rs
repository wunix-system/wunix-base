//! Работа с конфигурационными файлами служб Wunix (`/etc/services/*.conf`).

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use libwunix::{close, lstat, mkdir, open, read, symlink, unlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, Stat};

pub const SERVICES_DIR: &str = "/etc/services";
pub const ENABLED_DIR: &str = "/etc/services/enabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub description: String,
    pub command: String,
    pub autostart: bool,
    pub restart: RestartPolicy,
    pub user: String,
    pub directory: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            command: String::new(),
            autostart: false,
            restart: RestartPolicy::No,
            user: String::from("root"),
            directory: String::from("/"),
        }
    }
}

pub fn get_service_path(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    let resolved = if clean == "ssh" && !file_exists(&format!("{}/ssh.conf", SERVICES_DIR)) {
        "sshd"
    } else {
        clean
    };
    format!("{}/{}.conf", SERVICES_DIR, resolved)
}

pub fn get_enabled_path(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    format!("{}/{}.conf", ENABLED_DIR, clean)
}

pub fn file_exists(path: &str) -> bool {
    let mut null_path = [0u8; 256];
    let b = path.as_bytes();
    if b.len() >= null_path.len() {
        return false;
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let mut st = Stat::zeroed();
    lstat(&null_path[..b.len() + 1], &mut st) == 0
}

pub fn is_enabled(name: &str) -> bool {
    let path = get_enabled_path(name);
    file_exists(&path)
}

pub fn enable_service(name: &str) -> Result<(), &'static str> {
    let service_file = get_service_path(name);
    if !file_exists(&service_file) {
        return Err("файл конфигурации службы не найден");
    }

    ensure_dirs();

    let enabled_file = get_enabled_path(name);
    let mut null_target = [0u8; 256];
    let mut null_link = [0u8; 256];

    let tb = service_file.as_bytes();
    let lb = enabled_file.as_bytes();
    if tb.len() >= null_target.len() || lb.len() >= null_link.len() {
        return Err("путь к службе слишком длинный");
    }
    null_target[..tb.len()].copy_from_slice(tb);
    null_target[tb.len()] = 0;
    null_link[..lb.len()].copy_from_slice(lb);
    null_link[lb.len()] = 0;

    // Удаляем старую ссылку/файл, если существовал
    let _ = unlink(&null_link[..lb.len() + 1]);

    if symlink(&null_target[..tb.len() + 1], &null_link[..lb.len() + 1]) < 0 {
        // Если symlink не удался, создаем копию-маркер файла
        let sfd = open(&null_target[..tb.len() + 1], O_RDONLY as u64, 0);
        if sfd >= 0 {
            let dfd = open(&null_link[..lb.len() + 1], (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
            if dfd >= 0 {
                let mut buf = [0u8; 512];
                loop {
                    let n = read(sfd as u64, &mut buf);
                    if n == 0 {
                        break;
                    }
                    write(dfd as u64, &buf[..n as usize]);
                }
                close(dfd as u64);
            }
            close(sfd as u64);
        } else {
            return Err("ошибка создания ссылки в каталоге enabled");
        }
    }

    Ok(())
}

pub fn disable_service(name: &str) -> Result<(), &'static str> {
    let enabled_file = get_enabled_path(name);
    let mut null_link = [0u8; 256];
    let lb = enabled_file.as_bytes();
    if lb.len() >= null_link.len() {
        return Err("путь к службе слишком длинный");
    }
    null_link[..lb.len()].copy_from_slice(lb);
    null_link[lb.len()] = 0;

    let res = unlink(&null_link[..lb.len() + 1]);
    if res < 0 && !file_exists(&enabled_file) {
        return Ok(());
    }
    if res < 0 {
        return Err("не удалось отключить автозапуск службы");
    }
    Ok(())
}

pub fn load_service(name: &str) -> Result<ServiceConfig, &'static str> {
    let path = get_service_path(name);
    let mut null_path = [0u8; 256];
    let b = path.as_bytes();
    if b.len() >= null_path.len() {
        return Err("путь к файлу службы слишком длинный");
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], O_RDONLY as u64, 0);
    if fd < 0 {
        return Err("служба не найдена в /etc/services");
    }

    let mut content = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = read(fd as u64, &mut buf);
        if n == 0 {
            break;
        }
        content.extend_from_slice(&buf[..n as usize]);
        if content.len() > 64 * 1024 {
            break;
        }
    }
    close(fd as u64);

    let text = core::str::from_utf8(&content).map_err(|_| "некорректный utf-8 в файле конфигурации")?;
    parse_service_config(name, text)
}

pub fn parse_service_config(name: &str, text: &str) -> Result<ServiceConfig, &'static str> {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let mut config = ServiceConfig::default();
    config.name = String::from(clean_name);
    config.autostart = is_enabled(clean_name);

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with("//") {
            continue;
        }

        let mut parts = trimmed.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value = parts.next().unwrap_or("").trim();

        if key.eq_ignore_ascii_case("description") || key.eq_ignore_ascii_case("desc") {
            config.description = String::from(value);
        } else if key.eq_ignore_ascii_case("command") || key.eq_ignore_ascii_case("exec") || key.eq_ignore_ascii_case("cmd") {
            config.command = String::from(value);
        } else if key.eq_ignore_ascii_case("autostart") || key.eq_ignore_ascii_case("auto") {
            let v = value.to_ascii_lowercase();
            if v == "yes" || v == "true" || v == "1" || v == "on" {
                config.autostart = true;
            } else if v == "no" || v == "false" || v == "0" || v == "off" {
                config.autostart = false;
            }
        } else if key.eq_ignore_ascii_case("restart") {
            let v = value.to_ascii_lowercase();
            if v == "always" || v == "yes" || v == "true" {
                config.restart = RestartPolicy::Always;
            } else if v == "on-failure" || v == "on_failure" {
                config.restart = RestartPolicy::OnFailure;
            } else {
                config.restart = RestartPolicy::No;
            }
        } else if key.eq_ignore_ascii_case("user") {
            config.user = String::from(value);
        } else if key.eq_ignore_ascii_case("directory") || key.eq_ignore_ascii_case("workdir") {
            config.directory = String::from(value);
        }
    }

    if config.command.is_empty() {
        return Err("в конфигурации службы не указана команда запуска (command = ...)");
    }

    Ok(config)
}

pub fn create_template(name: &str) -> String {
    let clean = name.strip_suffix(".conf").unwrap_or(name);
    format!(
        "# Конфигурация системной службы Wunix\n\
         # Имя службы: {0}\n\n\
         # Понятное описание назначения службы\n\
         description = Служба {0}\n\n\
         # Исполняемый файл и параметры запуска\n\
         command = /bin/{0}\n\n\
         # Автозапуск при загрузке системы (yes / no)\n\
         autostart = yes\n\n\
         # Перезапуск при сбое (on-failure / always / no)\n\
         restart = on-failure\n\n\
         # Пользователь, от которого запускается процесс\n\
         user = root\n\n\
         # Рабочий каталог службы\n\
         directory = /\n",
        clean
    )
}

pub fn ensure_dirs() {
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/services\0", 0o755);
    let _ = mkdir(b"/etc/services/enabled\0", 0o755);
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/services\0", 0o755);
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/log\0", 0o755);
    let _ = mkdir(b"/var/log/services\0", 0o755);
}
