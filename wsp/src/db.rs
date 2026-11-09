//! Модуль локальной базы данных установленных системных пакетов WSP.
//!
//! Отвечает за хранение реестра `/var/lib/wsp/installed.json`,
//! проверку целостности установленных файлов и ведение журнала транзакций.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use libwunix::{close, mkdir, open, read, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

pub const WSP_DB_DIR: &[u8] = b"/var/lib/wsp\0";
pub const WSP_DB_PATH: &[u8] = b"/var/lib/wsp/installed.json\0";
pub const WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPackageEntry {
    pub name: String,
    pub version: String,
    pub tier: String,
    pub description: String,
    pub files: Vec<String>,
}

pub fn ensure_db_dirs() {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/lib\0", 0o755);
    let _ = mkdir(WSP_DB_DIR, 0o755);
    let _ = mkdir(b"/var/cache\0", 0o755);
    let _ = mkdir(b"/var/cache/wsp\0", 0o755);
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/wsp\0", 0o755);
}

pub fn has_critical_alert() -> bool {
    let fd = open(WSP_ALERT_PATH, O_RDONLY as u64, 0);
    if fd >= 0 {
        close(fd as u64);
        true
    } else {
        false
    }
}

pub fn load_installed_packages() -> Vec<SystemPackageEntry> {
    ensure_db_dirs();
    let fd = open(WSP_DB_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        let defaults = get_default_base_packages();
        save_installed_packages(&defaults);
        return defaults;
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = read(fd as u64, &mut chunk);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }
    close(fd as u64);

    if buf.is_empty() {
        let defaults = get_default_base_packages();
        save_installed_packages(&defaults);
        return defaults;
    }

    let content = core::str::from_utf8(&buf).unwrap_or("");
    let parsed = parse_installed_db(content);
    if parsed.is_empty() {
        let defaults = get_default_base_packages();
        save_installed_packages(&defaults);
        defaults
    } else {
        parsed
    }
}

pub fn save_installed_packages(packages: &[SystemPackageEntry]) {
    ensure_db_dirs();
    let fd = open(WSP_DB_PATH, (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
    if fd < 0 {
        return;
    }

    let _ = write(fd as u64, b"[\n");
    for (i, pkg) in packages.iter().enumerate() {
        let comma = if i + 1 < packages.len() { "," } else { "" };
        let entry = format!(
            "  {{\n    \"name\": \"{}\",\n    \"version\": \"{}\",\n    \"tier\": \"{}\",\n    \"description\": \"{}\",\n    \"files\": [\n",
            pkg.name, pkg.version, pkg.tier, pkg.description
        );
        let _ = write(fd as u64, entry.as_bytes());

        for (fi, file) in pkg.files.iter().enumerate() {
            let fcomma = if fi + 1 < pkg.files.len() { "," } else { "" };
            let fline = format!("      \"{}\"{}\n", file, fcomma);
            let _ = write(fd as u64, fline.as_bytes());
        }

        let footer = format!("    ]\n  }}{}\n", comma);
        let _ = write(fd as u64, footer.as_bytes());
    }
    let _ = write(fd as u64, b"]\n");
    close(fd as u64);
}

pub fn get_default_base_packages() -> Vec<SystemPackageEntry> {
    vec![
        SystemPackageEntry {
            name: "wunix-kernel".to_string(),
            version: "1.0.0".to_string(),
            tier: "critical".to_string(),
            description: "Монолитное ядро Wunix с VFS и вытесняющим планировщиком процессов".to_string(),
            files: vec![
                "/boot/wunix-kernel".to_string(),
                "/boot/limine.conf".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libwunix".to_string(),
            version: "1.0.0".to_string(),
            tier: "critical".to_string(),
            description: "Базовый системный рантайм и API системных вызовов Ring 3".to_string(),
            files: vec![
                "/usr/lib/libwunix.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libpackwunix".to_string(),
            version: "1.0.0".to_string(),
            tier: "critical".to_string(),
            description: "Библиотека контейнеров .wp и управления пакетами WSP/WPM".to_string(),
            files: vec![
                "/usr/lib/libpackwunix.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libcwunix".to_string(),
            version: "1.0.0".to_string(),
            tier: "base".to_string(),
            description: "Стандартная библиотека Си libc для программ Wunix".to_string(),
            files: vec![
                "/usr/lib/libc.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "wunix-base".to_string(),
            version: "1.0.0".to_string(),
            tier: "base".to_string(),
            description: "Базовый системный образ Wunix OS (init, wsh, wsp, wpm, wspd, shell)".to_string(),
            files: vec![
                "/bin/init".to_string(),
                "/bin/wsh".to_string(),
                "/bin/system".to_string(),
                "/bin/wsp".to_string(),
                "/bin/wspd".to_string(),
                "/bin/wpm".to_string(),
            ],
        },
    ]
}

fn parse_installed_db(content: &str) -> Vec<SystemPackageEntry> {
    let mut list = Vec::new();
    let mut cur_name = String::new();
    let mut cur_version = String::new();
    let mut cur_tier = String::new();
    let mut cur_desc = String::new();
    let mut cur_files = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        if trimmed.starts_with("\"name\":") {
            if let Some(val) = extract_json_val(trimmed) {
                cur_name = val;
            }
        } else if trimmed.starts_with("\"version\":") {
            if let Some(val) = extract_json_val(trimmed) {
                cur_version = val;
            }
        } else if trimmed.starts_with("\"tier\":") {
            if let Some(val) = extract_json_val(trimmed) {
                cur_tier = val;
            }
        } else if trimmed.starts_with("\"description\":") {
            if let Some(val) = extract_json_val(trimmed) {
                cur_desc = val;
            }
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            if let Some(val) = extract_json_val(trimmed) {
                if !val.is_empty() && val.starts_with('/') {
                    cur_files.push(val);
                }
            }
        } else if trimmed == "}" || trimmed == "}," {
            if !cur_name.is_empty() {
                list.push(SystemPackageEntry {
                    name: cur_name.clone(),
                    version: if cur_version.is_empty() { "1.0.0".to_string() } else { cur_version.clone() },
                    tier: if cur_tier.is_empty() { "base".to_string() } else { cur_tier.clone() },
                    description: cur_desc.clone(),
                    files: cur_files.clone(),
                });
            }
            cur_name.clear();
            cur_version.clear();
            cur_tier.clear();
            cur_desc.clear();
            cur_files.clear();
        }
    }

    list
}

fn extract_json_val(line: &str) -> Option<String> {
    let mut quote_indices = Vec::new();
    for (i, b) in line.bytes().enumerate() {
        if b == b'"' {
            quote_indices.push(i);
        }
    }
    if quote_indices.len() >= 4 {
        let start = quote_indices[2] + 1;
        let end = quote_indices[3];
        if end >= start {
            return Some(line[start..end].to_string());
        }
    } else if quote_indices.len() >= 2 {
        let start = quote_indices[0] + 1;
        let end = quote_indices[1];
        if end >= start {
            return Some(line[start..end].to_string());
        }
    }
    None
}

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
