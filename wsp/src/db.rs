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
use libpackwunix::sources::{parse_repo_index, RepoPackage};

pub const WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPackageEntry {
    pub name: String,
    pub version: String,
    pub tier: String,
    pub description: String,
    pub files: Vec<String>,
}

pub(crate) fn get_path(root: &str, sub: &str) -> String {
    if root.is_empty() {
        sub.to_string()
    } else {
        format!("{}{}", root.trim_end_matches('/'), sub)
    }
}

pub(crate) fn open_c(path: &str, flags: u64, mode: u64) -> i64 {
    let mut null_p = [0u8; 256];
    let b = path.as_bytes();
    let l = b.len().min(255);
    null_p[..l].copy_from_slice(&b[..l]);
    null_p[l] = 0;
    open(&null_p, flags, mode)
}

pub fn ensure_db_dirs() {
    ensure_db_dirs_in("");
}

pub fn ensure_db_dirs_in(root: &str) {
    let dirs = [
        "/var",
        "/var/lib",
        "/var/lib/wsp",
        "/var/cache",
        "/var/cache/wsp",
        "/run",
        "/run/wsp",
    ];
    for d in &dirs {
        let full = get_path(root, d);
        let mut null_p = [0u8; 256];
        let b = full.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let _ = mkdir(&null_p, 0o755);
    }
}

pub fn ensure_dir_recursive(path: &str) {
    let mut current = String::new();
    if path.starts_with('/') {
        current.push('/');
    }
    for comp in path.split('/') {
        if comp.is_empty() {
            continue;
        }
        if !current.ends_with('/') {
            current.push('/');
        }
        current.push_str(comp);
        let mut null_p = [0u8; 256];
        let b = current.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let _ = mkdir(&null_p, 0o755);
    }
}

pub fn copy_file(src: &str, dst: &str) -> bool {
    let mut null_s = [0u8; 256];
    let b = src.as_bytes();
    let l = b.len().min(255);
    null_s[..l].copy_from_slice(&b[..l]);
    null_s[l] = 0;

    let in_fd = open(&null_s, O_RDONLY as u64, 0);
    if in_fd < 0 {
        return false;
    }

    let mut null_d = [0u8; 256];
    let b = dst.as_bytes();
    let l = b.len().min(255);
    null_d[..l].copy_from_slice(&b[..l]);
    null_d[l] = 0;

    let mut out_fd = open(&null_d, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o666);
    if out_fd < 0 {
        let _ = libwunix::unlink(&null_d);
        out_fd = open(&null_d, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o666);
    }
    if out_fd < 0 {
        close(in_fd as u64);
        return false;
    }

    let mut buf = [0u8; 4096];
    loop {
        let n = read(in_fd as u64, &mut buf);
        if n <= 0 {
            break;
        }
        let w = write(out_fd as u64, &buf[..n as usize]);
        if w <= 0 {
            break;
        }
    }

    close(in_fd as u64);
    close(out_fd as u64);
    true
}

pub fn has_critical_alert() -> bool {
    has_critical_alert_in("")
}

pub fn has_critical_alert_in(root: &str) -> bool {
    let path = get_path(root, "/run/wsp/critical_alert.json");
    let fd = open_c(&path, O_RDONLY as u64, 0);
    if fd >= 0 {
        close(fd as u64);
        true
    } else {
        false
    }
}

pub fn load_installed_packages() -> Vec<SystemPackageEntry> {
    load_installed_packages_from("")
}

pub fn load_installed_packages_from(root: &str) -> Vec<SystemPackageEntry> {
    ensure_db_dirs_in(root);
    let db_path = get_path(root, "/var/lib/wsp/installed.json");
    let fd = open_c(&db_path, O_RDONLY as u64, 0);
    if fd < 0 {
        if root.is_empty() || root == "/" {
            let defaults = get_default_base_packages();
            save_installed_packages_to(root, &defaults);
            return defaults;
        } else {
            return Vec::new();
        }
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
        if root.is_empty() || root == "/" {
            let defaults = get_default_base_packages();
            save_installed_packages_to(root, &defaults);
            return defaults;
        } else {
            return Vec::new();
        }
    }

    let content = core::str::from_utf8(&buf).unwrap_or("");
    let mut parsed = parse_installed_db(content);
    if parsed.is_empty() {
        if root.is_empty() || root == "/" {
            let defaults = get_default_base_packages();
            save_installed_packages_to(root, &defaults);
            defaults
        } else {
            Vec::new()
        }
    } else {
        if !parsed.iter().any(|p| p.name == "wunix-kernel") {
            let kpath = get_path(root, "/boot/wunix-kernel");
            let kfd = open_c(&kpath, O_RDONLY as u64, 0);
            if kfd >= 0 {
                close(kfd as u64);
                parsed.insert(
                    0,
                    SystemPackageEntry {
                        name: "wunix-kernel".to_string(),
                        version: "0.1.0".to_string(),
                        tier: "critical".to_string(),
                        description: "Монолитное ядро Wunix с VFS и вытесняющим планировщиком процессов".to_string(),
                        files: alloc::vec![
                            "/boot/wunix-kernel".to_string(),
                            "/boot/limine.conf".to_string(),
                        ],
                    },
                );
                save_installed_packages_to(root, &parsed);
            }
        }
        parsed
    }
}

#[allow(dead_code)]
pub fn save_installed_packages(packages: &[SystemPackageEntry]) {
    save_installed_packages_to("", packages);
}

pub fn save_installed_packages_to(root: &str, packages: &[SystemPackageEntry]) {
    ensure_db_dirs_in(root);
    let db_path = get_path(root, "/var/lib/wsp/installed.json");
    let fd = open_c(&db_path, (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
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

pub fn load_repo_packages(root: &str) -> Vec<RepoPackage> {
    let candidates = [
        get_path(root, "/var/lib/wsp/packages.json"),
        get_path(root, "/var/lib/wsp/repo/system/x86_64/packages.json"),
        get_path(root, "/var/cache/wsp/packages.json"),
        "/var/lib/wsp/packages.json".to_string(),
        "/var/lib/wsp/repo/system/x86_64/packages.json".to_string(),
        "/var/cache/wsp/packages.json".to_string(),
    ];

    for path in &candidates {
        let fd = open_c(path, O_RDONLY as u64, 0);
        if fd >= 0 {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = read(fd as u64, &mut chunk);
                if n <= 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n as usize]);
            }
            close(fd as u64);

            if let Ok(content) = core::str::from_utf8(&buf) {
                let pkgs = parse_repo_index(content);
                if !pkgs.is_empty() {
                    return pkgs;
                }
            }
        }
    }

    Vec::new()
}

pub fn get_default_base_packages() -> Vec<SystemPackageEntry> {
    vec![
        SystemPackageEntry {
            name: "wunix-kernel".to_string(),
            version: "0.1.0".to_string(),
            tier: "critical".to_string(),
            description: "Монолитное ядро Wunix с VFS и вытесняющим планировщиком процессов".to_string(),
            files: vec![
                "/boot/wunix-kernel".to_string(),
                "/boot/limine.conf".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libwunix".to_string(),
            version: "0.1.0".to_string(),
            tier: "critical".to_string(),
            description: "Базовый системный рантайм и API системных вызовов Ring 3".to_string(),
            files: vec![
                "/usr/lib/libwunix.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libpackwunix".to_string(),
            version: "0.1.0".to_string(),
            tier: "critical".to_string(),
            description: "Библиотека контейнеров .wp и управления пакетами WSP/WPM".to_string(),
            files: vec![
                "/usr/lib/libpackwunix.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "libcwunix".to_string(),
            version: "0.1.0".to_string(),
            tier: "base".to_string(),
            description: "Стандартная библиотека Си libc для программ Wunix".to_string(),
            files: vec![
                "/usr/lib/libc.a".to_string(),
            ],
        },
        SystemPackageEntry {
            name: "wunix-base".to_string(),
            version: "0.1.0".to_string(),
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
