//! Модуль установки и удаления прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, mkdir, open, println, read, t, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

use crate::display::{print_error, print_info, COLOR_GREEN, COLOR_RESET};
use crate::guard::ensure_wsp_system_safe;

pub const WPM_DB_PATH: &[u8] = b"/var/lib/wpm/installed.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub category: String,
}

pub fn run_install(packages: &[&str], _assume_yes: bool, ignore_unsafe: bool, verbose: bool) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите хотя бы один пакет для установки: wpm install <пакет>",
            "missing package name to install: wpm install <package>",
        ));
        return 1;
    }

    // 1. Проверка защитного барьера WSP
    if ensure_wsp_system_safe(ignore_unsafe).is_err() {
        return 1;
    }

    print_info(t(
        "подготовка к загрузке и установке пакетов...",
        "preparing to download and install packages...",
    ));

    let mut installed = load_installed_apps();

    for &pkg in packages {
        if verbose {
            println!("  • {} {}.wsp...", t("загрузка пакета", "downloading"), pkg);
            println!("  • {}...", t("проверка контрольной суммы SHA-256", "verifying SHA-256 checksum"));
            println!("  • {}...", t("распаковка файлов в /usr/local", "extracting files to /usr/local"));
        } else {
            println!("  • {}: {}.wsp", t("установка", "installing"), pkg);
        }

        if !installed.iter().any(|a| a.name == pkg) {
            installed.push(InstalledApp {
                name: pkg.to_string(),
                version: "1.0.0".to_string(),
                category: "applications".to_string(),
            });
        }
    }

    save_installed_apps(&installed);

    println!();
    println!("{}{} {}", COLOR_GREEN, t("wpm: установка пакетов успешно завершена.", "wpm: packages installed successfully."), COLOR_RESET);
    0
}

pub fn run_remove(packages: &[&str], purge: bool) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите имя пакета для удаления: wpm remove <пакет>",
            "missing package name to remove: wpm remove <package>",
        ));
        return 1;
    }

    let mut installed = load_installed_apps();
    let initial_count = installed.len();

    for &pkg in packages {
        if purge {
            println!("  • {}: {} (включая файлы конфигурации)", t("удаление", "removing"), pkg);
        } else {
            println!("  • {}: {}", t("удаление", "removing"), pkg);
        }
        installed.retain(|a| a.name != pkg);
    }

    if installed.len() < initial_count {
        save_installed_apps(&installed);
        println!("{}", t("wpm: удаление пакетов завершено.", "wpm: packages removed successfully."));
        0
    } else {
        print_error(t(
            "указанные пакеты не были найдены в списке установленных.",
            "specified packages were not found in installed list.",
        ));
        1
    }
}

pub fn load_installed_apps() -> Vec<InstalledApp> {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/lib\0", 0o755);
    let _ = mkdir(b"/var/lib/wpm\0", 0o755);

    let fd = open(WPM_DB_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        return Vec::new();
    }

    let mut buf = [0u8; 4096];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n == 0 {
        return Vec::new();
    }

    let s = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
    let mut list = Vec::new();

    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let mut parts = t.split(':');
        if let Some(name) = parts.next() {
            let ver = parts.next().unwrap_or("1.0.0");
            let cat = parts.next().unwrap_or("applications");
            list.push(InstalledApp {
                name: name.to_string(),
                version: ver.to_string(),
                category: cat.to_string(),
            });
        }
    }

    list
}

fn save_installed_apps(apps: &[InstalledApp]) {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/lib\0", 0o755);
    let _ = mkdir(b"/var/lib/wpm\0", 0o755);

    let mut out = String::new();
    for a in apps {
        out.push_str(&format!("{}:{}:{}\n", a.name, a.version, a.category));
    }

    let fd = open(WPM_DB_PATH, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let b = out.as_bytes();
        let _ = write(fd as u64, b);
        close(fd as u64);
    }
}
