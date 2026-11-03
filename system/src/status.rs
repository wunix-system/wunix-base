//! Человечный, лаконичный и понятный вывод статуса системных служб Wunix.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libwunix::{close, getdents64, open, println, t, O_RDONLY};
use crate::config::{is_enabled, load_service, SERVICES_DIR};
use crate::logs::print_last_lines;
use crate::service::{get_log_path, load_runtime_status, ServiceState};

pub fn show_service_status(name: &str) {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let config = match load_service(clean_name) {
        Ok(c) => c,
        Err(e) => {
            println!(
                "{} '{}': {}",
                t("Служба", "Service"),
                clean_name,
                e
            );
            return;
        }
    };

    let runtime = load_runtime_status(clean_name);
    let enabled = is_enabled(clean_name);

    let (icon, state_colored) = match runtime.state {
        ServiceState::Running => (
            "\x1b[1;32m●\x1b[0m",
            t("\x1b[1;32mработает\x1b[0m", "\x1b[1;32mrunning\x1b[0m"),
        ),
        ServiceState::Finished(code) => {
            if code == 0 {
                (
                    "\x1b[1;32m●\x1b[0m",
                    t("\x1b[1;32mработает\x1b[0m", "\x1b[1;32mrunning\x1b[0m"),
                )
            } else {
                (
                    "\x1b[1;31m×\x1b[0m",
                    t("\x1b[1;31mошибка запуска\x1b[0m", "\x1b[1;31merror\x1b[0m"),
                )
            }
        }
        ServiceState::Failed(_code) => (
            "\x1b[1;31m×\x1b[0m",
            t("\x1b[1;31mошибка\x1b[0m", "\x1b[1;31mfailed\x1b[0m"),
        ),
        ServiceState::Stopped => (
            "\x1b[1;33m○\x1b[0m",
            t("\x1b[1;33mостановлена\x1b[0m", "\x1b[1;33mstopped\x1b[0m"),
        ),
        ServiceState::Unknown => (
            "\x1b[1;90m○\x1b[0m",
            t("\x1b[1;90mостановлена\x1b[0m", "\x1b[1;90mstopped\x1b[0m"),
        ),
    };

    let auto_str = if enabled {
        t("включен", "enabled")
    } else {
        t("отключен", "disabled")
    };

    let desc = if config.description.is_empty() {
        clean_name
    } else {
        &config.description
    };

    println!("\n{} \x1b[1m{}\x1b[0m — {}", icon, clean_name, desc);
    println!("  • Состояние:   {}", state_colored);
    println!("  • Автозапуск:  {}", auto_str);
    println!("  • Команда:     {}", config.command);
    println!("  • Файл логов:  {}", get_log_path(clean_name));

    if let ServiceState::Failed(code) = runtime.state {
        println!("  • Код ошибки:  {}", code);
    }

    println!("\n{}", t("Последние записи:", "Recent log output:"));
    print_last_lines(clean_name, 5);
    println!();
}

pub fn list_all_services() {
    let mut null_path = [0u8; 64];
    let b = SERVICES_DIR.as_bytes();
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], O_RDONLY as u64, 0);
    if fd < 0 {
        println!("{}", t("Службы не найдены (/etc/services отсутствует).", "No services found (/etc/services missing)."));
        return;
    }

    let mut buf = [0u8; 4096];
    let mut names: Vec<String> = Vec::new();

    loop {
        let n = getdents64(fd as u64, &mut buf);
        if n <= 0 {
            break;
        }

        let total = (n as usize).min(buf.len());
        let mut pos = 0;
        while pos + 19 <= total {
            let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
            if reclen == 0 || pos + reclen > total {
                break;
            }

            let name_start = pos + 19;
            let mut name_end = name_start;
            let max_end = pos + reclen;
            while name_end < max_end && buf[name_end] != 0 {
                name_end += 1;
            }

            if name_end > name_start {
                if let Ok(entry_name) = core::str::from_utf8(&buf[name_start..name_end]) {
                    if entry_name.ends_with(".conf") && !entry_name.starts_with('.') {
                        if let Some(clean) = entry_name.strip_suffix(".conf") {
                            names.push(String::from(clean));
                        }
                    }
                }
            }

            pos += reclen;
        }
    }
    close(fd as u64);

    if names.is_empty() {
        println!("{}", t("В системе пока нет настроенных служб. Создать новую: system create <имя>", "No services configured yet. Create one: system create <name>"));
        return;
    }

    names.sort();

    println!("\n\x1b[1m{}\x1b[0m", t("Службы Wunix:", "Wunix Services:"));

    for name in &names {
        let cfg = load_service(name).unwrap_or_default();
        let runtime = load_runtime_status(name);
        let enabled = is_enabled(name);

        let (icon, state_text) = match runtime.state {
            ServiceState::Running => ("\x1b[1;32m●\x1b[0m", "\x1b[32mработает\x1b[0m   "),
            ServiceState::Finished(code) => {
                if code == 0 {
                    ("\x1b[1;32m●\x1b[0m", "\x1b[32mработает\x1b[0m   ")
                } else {
                    ("\x1b[1;31m×\x1b[0m", "\x1b[31mошибка\x1b[0m     ")
                }
            }
            ServiceState::Failed(_) => ("\x1b[1;31m×\x1b[0m", "\x1b[31mошибка\x1b[0m     "),
            ServiceState::Stopped | ServiceState::Unknown => ("\x1b[1;90m○\x1b[0m", "\x1b[90mостановлена\x1b[0m"),
        };

        let auto_tag = if enabled {
            "\x1b[32m[автозапуск]\x1b[0m"
        } else {
            "            "
        };

        let desc = if cfg.description.is_empty() {
            name.as_str()
        } else {
            &cfg.description
        };

        println!("  {} \x1b[1m{:<11}\x1b[0m {} {}  {}", icon, name, state_text, auto_tag, desc);
    }

    println!("\n\x1b[90m{}\x1b[0m\n", t(
        "Управление: system start|stop|restart|status|logs|edit|create <служба>",
        "Commands: system start|stop|restart|status|logs|edit|create <service>"
    ));
}
