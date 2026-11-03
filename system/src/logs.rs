//! Чтение и отслеживание журналов служб Wunix (`/var/log/services/<name>.log`).

extern crate alloc;
use alloc::vec::Vec;
use libwunix::{close, open, pread64, print, println, t, time::sleep_ms, O_RDONLY};
use crate::service::get_log_path;

pub fn show_logs(name: &str, tail: Option<usize>, head: Option<usize>, follow: bool) {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let log_path = get_log_path(clean_name);

    let mut null_path = [0u8; 256];
    let b = log_path.as_bytes();
    if b.len() >= null_path.len() {
        println!("{}", t("system: слишком длинный путь к журналу", "system: log path too long"));
        return;
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], O_RDONLY as u64, 0);
    if fd < 0 {
        println!(
            "{} '{}' ({}).",
            t("system: журнал службы пуст или еще не создан для", "system: log file is empty or not created yet for"),
            clean_name,
            log_path
        );
        return;
    }

    let mut content = Vec::new();
    let mut buf = [0u8; 4096];
    let mut offset = 0u64;

    loop {
        let n = pread64(fd as u64, &mut buf, offset);
        if n <= 0 {
            break;
        }
        content.extend_from_slice(&buf[..n as usize]);
        offset += n as u64;
    }

    let text = core::str::from_utf8(&content).unwrap_or("");
    let all_lines: Vec<&str> = text.lines().collect();

    if let Some(h) = head {
        let count = h.min(all_lines.len());
        for line in &all_lines[..count] {
            println!("{}", line);
        }
    } else if let Some(t_count) = tail {
        let start = all_lines.len().saturating_sub(t_count);
        for line in &all_lines[start..] {
            println!("{}", line);
        }
    } else {
        for line in &all_lines {
            println!("{}", line);
        }
    }

    if follow {
        println!(
            "\x1b[90m--- {} '{}' (Ctrl+C для выхода) ---\x1b[0m",
            t("Отслеживание журнала службы", "Following logs of service"),
            clean_name
        );

        let mut read_buf = [0u8; 1024];
        loop {
            let n = pread64(fd as u64, &mut read_buf, offset);
            if n > 0 {
                if let Ok(new_text) = core::str::from_utf8(&read_buf[..n as usize]) {
                    print!("{}", new_text);
                }
                offset += n as u64;
            } else {
                sleep_ms(250);
            }
        }
    }

    close(fd as u64);
}

pub fn print_last_lines(name: &str, count: usize) {
    let clean_name = name.strip_suffix(".conf").unwrap_or(name);
    let log_path = get_log_path(clean_name);

    let mut null_path = [0u8; 256];
    let b = log_path.as_bytes();
    if b.len() >= null_path.len() {
        return;
    }
    null_path[..b.len()].copy_from_slice(b);
    null_path[b.len()] = 0;

    let fd = open(&null_path[..b.len() + 1], O_RDONLY as u64, 0);
    if fd < 0 {
        println!("   \x1b[90m(нет записей в журнале)\x1b[0m");
        return;
    }

    let mut content = Vec::new();
    let mut buf = [0u8; 4096];
    let mut offset = 0u64;

    loop {
        let n = pread64(fd as u64, &mut buf, offset);
        if n <= 0 {
            break;
        }
        content.extend_from_slice(&buf[..n as usize]);
        offset += n as u64;
        if content.len() > 64 * 1024 {
            break;
        }
    }
    close(fd as u64);

    let text = core::str::from_utf8(&content).unwrap_or("");
    let all_lines: Vec<&str> = text.lines().collect();

    if all_lines.is_empty() {
        println!("   \x1b[90m(журнал пуст)\x1b[0m");
        return;
    }

    let start = all_lines.len().saturating_sub(count);
    for line in &all_lines[start..] {
        println!("   {}", line);
    }
}
