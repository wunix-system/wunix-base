//! Модуль локальной базы данных установленных системных пакетов WSP.
//!
//! Отвечает за хранение реестра `/var/lib/wsp/installed.json`,
//! проверку целостности установленных файлов и ведение журнала транзакций.

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use libwunix::{close, mkdir, open, read, write, O_CREAT, O_RDONLY, O_WRONLY};

pub const WSP_DB_DIR: &[u8] = b"/var/lib/wsp\0";
pub const WSP_DB_PATH: &[u8] = b"/var/lib/wsp/installed.json\0";
pub const WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

/// Запись об установленном системном пакете
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPackageEntry {
    pub name: String,
    pub version: String,
    pub tier: String,
    pub description: String,
    pub files: Vec<String>,
}

/// Инициализирует каталоги хранения базы данных WSP
pub fn ensure_db_dirs() {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/lib\0", 0o755);
    let _ = mkdir(WSP_DB_DIR, 0o755);
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/wsp\0", 0o755);
}

/// Проверяет наличие активного флага критической тревоги
pub fn has_critical_alert() -> bool {
    let fd = open(WSP_ALERT_PATH, O_RDONLY as u64, 0);
    if fd >= 0 {
        close(fd as u64);
        true
    } else {
        false
    }
}

/// Считывает список установленных системных пакетов
pub fn load_installed_packages() -> Vec<SystemPackageEntry> {
    ensure_db_dirs();
    let fd = open(WSP_DB_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        // Если база еще не создана, возвращаем базовый начальный набор ядра и wunix-base
        return get_default_base_packages();
    }

    let mut buf = [0u8; 8192];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n == 0 {
        return get_default_base_packages();
    }

    let content = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
    parse_installed_db(content)
}

/// Базовый список компонентов установленной системы по умолчанию
pub fn get_default_base_packages() -> Vec<SystemPackageEntry> {
    vec![
        SystemPackageEntry {
            name: "wunix-kernel".to_string(),
            version: "0.4.1".to_string(),
            tier: "critical".to_string(),
            description: "Монолитное ядро Wunix с VFS и вытесняющим планировщиком".to_string(),
            files: vec![
                "/boot/wunix-kernel".to_string(),
                "/boot/limine.conf".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libwunix".to_string(),
            version: "0.1.0".to_string(),
            tier: "critical".to_string(),
            description: "Системная библиотека системных вызовов Ring 3".to_string(),
            files: vec![
                "/lib/libwunix.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "wunix-base".to_string(),
            version: "1.0.0".to_string(),
            tier: "base".to_string(),
            description: "Базовый набор системных утилит (init, wsh, coreutils, net)".to_string(),
            files: vec![
                "/bin/init".to_string(),
                "/bin/wsh".to_string(),
                "/bin/ls".to_string(),
                "/bin/cat".to_string(),
                "/bin/ip".to_string(),
                "/bin/curl".to_string(),
                "/bin/nano".to_string(),
            ],
        },
    ]
}

/// Разбирает простую структуру базы установленных пакетов
fn parse_installed_db(content: &str) -> Vec<SystemPackageEntry> {
    let mut list = Vec::new();
    let mut cur_pkg: Option<SystemPackageEntry> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("package:") {
            if let Some(pkg) = cur_pkg.take() {
                list.push(pkg);
            }
            let name = trimmed["package:".len()..].trim();
            cur_pkg = Some(SystemPackageEntry {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                tier: "base".to_string(),
                description: String::new(),
                files: Vec::new(),
            });
            continue;
        }

        if let Some(ref mut pkg) = cur_pkg {
            if trimmed.starts_with("version:") {
                pkg.version = trimmed["version:".len()..].trim().to_string();
            } else if trimmed.starts_with("tier:") {
                pkg.tier = trimmed["tier:".len()..].trim().to_string();
            } else if trimmed.starts_with("description:") {
                pkg.description = trimmed["description:".len()..].trim().to_string();
            } else if trimmed.starts_with("file:") {
                pkg.files.push(trimmed["file:".len()..].trim().to_string());
            }
        }
    }

    if let Some(pkg) = cur_pkg {
        list.push(pkg);
    }

    if list.is_empty() {
        get_default_base_packages()
    } else {
        list
    }
}

/// Сохраняет журнал транзакции
pub fn log_transaction(action: &str, details: &str) {
    ensure_db_dirs();
    let journal_path = b"/var/lib/wsp/journal.log\0";
    let fd = open(journal_path, (O_WRONLY | O_CREAT) as u64, 0o644);
    if fd >= 0 {
        let msg = alloc::format!("[WSP] {}: {}\n", action, details);
        let _ = write(fd as u64, msg.as_bytes());
        close(fd as u64);
    }
}
