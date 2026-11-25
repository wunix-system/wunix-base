//! Единый модуль базы установленных пакетов и каталогов репозиториев WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, mkdir, open, read, write, O_APPEND, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
use libpackwunix::sources::{parse_repo_index, RepoPackage};

pub const WPM_ALERT_PATH: &[u8] = b"/run/wpm/critical_alert.json\0";
pub const LEGACY_WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    pub name: String,
    pub version: String,
    pub pkg_type: String,
    pub tier: String,
    pub is_meta: bool,
    pub description: String,
    pub dependencies: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub channel: String,
    pub pkg: RepoPackage,
}

pub(crate) fn get_path(root: &str, sub: &str) -> String {
    if root.is_empty() || root == "/" {
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

pub fn read_file_bytes(path: &str) -> Option<Vec<u8>> {
    let fd = open_c(path, O_RDONLY as u64, 0);
    if fd < 0 {
        return None;
    }
    let mut null_p = [0u8; 256];
    let b = path.as_bytes();
    let l = b.len().min(255);
    null_p[..l].copy_from_slice(&b[..l]);
    null_p[l] = 0;
    let mut st = libwunix::Stat::zeroed();
    let prealloc = if libwunix::stat(&null_p[..l + 1], &mut st) == 0 && st.st_size > 0 {
        (st.st_size as usize).min(32 * 1024 * 1024)
    } else {
        0
    };
    let mut buf = Vec::with_capacity(prealloc);
    let mut chunk = [0u8; 4096];
    loop {
        let n = read(fd as u64, &mut chunk);
        if n <= 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n as usize]);
    }
    close(fd as u64);
    Some(buf)
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

pub fn ensure_db_dirs_in(root: &str) {
    for d in &[
        "/var",
        "/var/lib",
        "/var/lib/wpm",
        "/var/lib/wpm/repo",
        "/var/lib/wpm/repo/system",
        "/var/lib/wpm/repo/system/x86_64",
        "/var/lib/wpm/repo/apps",
        "/var/lib/wpm/repo/apps/x86_64",
        "/var/cache",
        "/var/cache/wpm",
        "/run",
        "/run/wpm",
    ] {
        let full = get_path(root, d);
        let mut null_p = [0u8; 256];
        let b = full.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let _ = mkdir(&null_p, 0o755);
    }
}

pub fn copy_file(src: &str, dst: &str) -> bool {
    let in_fd = open_c(src, O_RDONLY as u64, 0);
    if in_fd < 0 {
        return false;
    }

    let mut out_fd = open_c(dst, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if out_fd < 0 {
        let mut null_d = [0u8; 256];
        let b = dst.as_bytes();
        let l = b.len().min(255);
        null_d[..l].copy_from_slice(&b[..l]);
        null_d[l] = 0;
        let _ = libwunix::unlink(&null_d);
        out_fd = open_c(dst, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
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
        if write(out_fd as u64, &buf[..n as usize]) <= 0 {
            break;
        }
    }

    close(in_fd as u64);
    close(out_fd as u64);
    true
}

pub fn has_critical_alert() -> bool {
    for p in &[WPM_ALERT_PATH, LEGACY_WSP_ALERT_PATH] {
        let fd = open(*p, O_RDONLY as u64, 0);
        if fd >= 0 {
            close(fd as u64);
            return true;
        }
    }
    false
}

pub fn clear_critical_alerts() {
    let _ = libwunix::unlink(WPM_ALERT_PATH);
    let _ = libwunix::unlink(LEGACY_WSP_ALERT_PATH);
}

pub fn load_installed_packages() -> Vec<PackageEntry> {
    load_installed_packages_from("")
}

pub fn load_installed_packages_from(root: &str) -> Vec<PackageEntry> {
    ensure_db_dirs_in(root);
    let db_path = get_path(root, "/var/lib/wpm/installed.json");
    if let Some(buf) = read_file_bytes(&db_path) {
        if let Ok(text) = core::str::from_utf8(&buf) {
            let parsed = parse_installed_db(text, "user");
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }

    // Миграция из устаревшего /var/lib/wsp/installed.json при наличии
    let legacy_path = get_path(root, "/var/lib/wsp/installed.json");
    if let Some(buf) = read_file_bytes(&legacy_path) {
        if let Ok(text) = core::str::from_utf8(&buf) {
            let parsed = parse_installed_db(text, "system");
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }

    Vec::new()
}

#[allow(dead_code)]
pub fn save_installed_packages(entries: &[PackageEntry]) {
    save_installed_packages_to("", entries);
}

pub fn save_installed_packages_to(root: &str, entries: &[PackageEntry]) {
    ensure_db_dirs_in(root);
    let db_path = get_path(root, "/var/lib/wpm/installed.json");
    let tmp_path = get_path(root, "/var/lib/wpm/installed.json.tmp");

    let mut null_tmp = [0u8; 256];
    let tb = tmp_path.as_bytes();
    let tl = tb.len().min(255);
    null_tmp[..tl].copy_from_slice(&tb[..tl]);
    null_tmp[tl] = 0;
    let _ = libwunix::unlink(&null_tmp[..tl + 1]);

    let fd = open(&null_tmp[..tl + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
    if fd < 0 {
        return;
    }

    let mut out_buf = String::with_capacity(8192);
    out_buf.push_str("[\n");
    for (i, pkg) in entries.iter().enumerate() {
        let comma = if i + 1 < entries.len() { "," } else { "" };
        let desc = pkg.description.replace('"', "\\\"");
        out_buf.push_str(&format!(
            "  {{\n    \"name\": \"{}\",\n    \"version\": \"{}\",\n    \"type\": \"{}\",\n    \"tier\": \"{}\",\n    \"meta\": {},\n    \"description\": \"{}\",\n    \"dependencies\": [\n",
            pkg.name, pkg.version, pkg.pkg_type, pkg.tier, if pkg.is_meta { "true" } else { "false" }, desc
        ));

        for (di, dep) in pkg.dependencies.iter().enumerate() {
            let dc = if di + 1 < pkg.dependencies.len() { "," } else { "" };
            out_buf.push_str(&format!("      \"{}\"{}\n", dep, dc));
        }
        out_buf.push_str("    ],\n    \"files\": [\n");

        for (fi, file) in pkg.files.iter().enumerate() {
            let fc = if fi + 1 < pkg.files.len() { "," } else { "" };
            out_buf.push_str(&format!("      \"{}\"{}\n", file, fc));
            if out_buf.len() >= 4096 {
                let _ = write(fd as u64, out_buf.as_bytes());
                out_buf.clear();
            }
        }
        out_buf.push_str(&format!("    ]\n  }}{}\n", comma));
        if out_buf.len() >= 4096 {
            let _ = write(fd as u64, out_buf.as_bytes());
            out_buf.clear();
        }
    }
    out_buf.push_str("]\n");
    if !out_buf.is_empty() {
        let _ = write(fd as u64, out_buf.as_bytes());
    }
    close(fd as u64);

    let mut null_db = [0u8; 256];
    let db_b = db_path.as_bytes();
    let db_l = db_b.len().min(255);
    null_db[..db_l].copy_from_slice(&db_b[..db_l]);
    null_db[db_l] = 0;
    if libwunix::rename(&null_tmp[..tl + 1], &null_db[..db_l + 1]) < 0 {
        let _ = libwunix::unlink(&null_db[..db_l + 1]);
        let _ = copy_file(&tmp_path, &db_path);
        let _ = libwunix::unlink(&null_tmp[..tl + 1]);
    }
}

pub fn load_system_catalog_from(root: &str) -> Vec<RepoPackage> {
    let candidates = [
        get_path(root, "/var/lib/wpm/repo/system/x86_64/packages.json"),
        get_path(root, "/repositories/system/x86_64/packages.json"),
        "/var/lib/wpm/repo/system/x86_64/packages.json".to_string(),
        "/repositories/system/x86_64/packages.json".to_string(),
    ];
    for path in &candidates {
        if let Some(buf) = read_file_bytes(path) {
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

pub fn load_user_catalog_from(root: &str) -> Vec<(String, RepoPackage)> {
    let sources = crate::repo_cmd::load_sources();
    let mut results: Vec<(String, RepoPackage)> = Vec::new();
    let mut loaded_channels: Vec<String> = Vec::new();

    for sec in &sources {
        let paths = [
            get_path(root, &format!("/var/lib/wpm/repo/{}/x86_64/packages.json", sec.name)),
            format!("/var/lib/wpm/repo/{}/x86_64/packages.json", sec.name),
        ];
        for path in &paths {
            if let Some(buf) = read_file_bytes(path) {
                if let Ok(content) = core::str::from_utf8(&buf) {
                    let pkgs = parse_repo_index(content);
                    if !pkgs.is_empty() {
                        for p in pkgs {
                            // Защита: пользовательский репозиторий не может подменять системные пакеты
                            if p.package_type != "system" && !results.iter().any(|(_, e)| e.name == p.name) {
                                results.push((sec.name.clone(), p));
                            }
                        }
                        loaded_channels.push(sec.name.clone());
                        break;
                    }
                }
            }
        }
    }

    if !loaded_channels.iter().any(|c| c == "apps") {
        let fallbacks = [
            get_path(root, "/repositories/apps/x86_64/packages.json"),
            "/repositories/apps/x86_64/packages.json".to_string(),
            "/var/cache/wpm/packages.json".to_string(),
        ];
        for path in &fallbacks {
            if let Some(buf) = read_file_bytes(path) {
                if let Ok(content) = core::str::from_utf8(&buf) {
                    let pkgs = parse_repo_index(content);
                    if !pkgs.is_empty() {
                        for p in pkgs {
                            if p.package_type != "system" && !results.iter().any(|(_, e)| e.name == p.name) {
                                results.push(("apps".to_string(), p));
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    results
}

pub fn load_unified_catalog_from(root: &str) -> Vec<CatalogEntry> {
    let mut out = Vec::new();
    for p in load_system_catalog_from(root) {
        out.push(CatalogEntry {
            channel: "system".to_string(),
            pkg: p,
        });
    }
    for (ch, p) in load_user_catalog_from(root) {
        if !out.iter().any(|e| e.pkg.name == p.name) {
            out.push(CatalogEntry { channel: ch, pkg: p });
        }
    }
    out
}

pub fn log_transaction(action: &str, details: &str) {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/log\0", 0o755);
    let fd = open(b"/var/log/wpm.log\0", (O_CREAT | O_WRONLY | O_APPEND) as u64, 0o644);
    if fd >= 0 {
        let ts = libwunix::time::clock_realtime().tv_sec;
        let line = format!("[{}] {}: {}\n", ts, action, details);
        let _ = write(fd as u64, line.as_bytes());
        close(fd as u64);
    }
}

enum ArraySection {
    None,
    Deps,
    Files,
}

fn parse_installed_db(content: &str, default_type: &str) -> Vec<PackageEntry> {
    let mut list = Vec::new();
    let mut cur_name = String::new();
    let mut cur_version = String::new();
    let mut cur_type = default_type.to_string();
    let mut cur_tier = "optional".to_string();
    let mut cur_meta = false;
    let mut cur_desc = String::new();
    let mut cur_deps = Vec::new();
    let mut cur_files = Vec::new();
    let mut arr = ArraySection::None;

    for line in content.lines() {
        let trimmed = line.trim().trim_end_matches(',');
        match arr {
            ArraySection::Deps => {
                if trimmed == "]" {
                    arr = ArraySection::None;
                } else {
                    extract_array_items(trimmed, &mut cur_deps);
                }
                continue;
            }
            ArraySection::Files => {
                if trimmed == "]" {
                    arr = ArraySection::None;
                } else {
                    extract_array_items(trimmed, &mut cur_files);
                }
                continue;
            }
            ArraySection::None => {}
        }

        if trimmed.starts_with("\"name\":") {
            if let Some(v) = extract_json_val(trimmed) {
                cur_name = v;
            }
        } else if trimmed.starts_with("\"version\":") {
            if let Some(v) = extract_json_val(trimmed) {
                cur_version = v;
            }
        } else if trimmed.starts_with("\"type\":") {
            if let Some(v) = extract_json_val(trimmed) {
                cur_type = v;
            }
        } else if trimmed.starts_with("\"tier\":") {
            if let Some(v) = extract_json_val(trimmed) {
                cur_tier = v;
            }
        } else if trimmed.starts_with("\"meta\":") {
            if let Some(colon) = trimmed.find(':') {
                cur_meta = trimmed[colon + 1..].trim() == "true";
            }
        } else if trimmed.starts_with("\"description\":") {
            if let Some(v) = extract_json_val(trimmed) {
                cur_desc = v;
            }
        } else if trimmed.starts_with("\"dependencies\":") {
            if let Some(br) = trimmed.find('[') {
                let rest = &trimmed[br + 1..];
                extract_array_items(rest, &mut cur_deps);
                if !rest.contains(']') {
                    arr = ArraySection::Deps;
                }
            }
        } else if trimmed.starts_with("\"files\":") {
            if let Some(br) = trimmed.find('[') {
                let rest = &trimmed[br + 1..];
                extract_array_items(rest, &mut cur_files);
                if !rest.contains(']') {
                    arr = ArraySection::Files;
                }
            }
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            if let Some(v) = extract_json_val(trimmed) {
                if v.starts_with('/') {
                    cur_files.push(v);
                }
            }
        } else if trimmed == "}" || trimmed == "}," {
            if !cur_name.is_empty() {
                list.push(PackageEntry {
                    name: cur_name.clone(),
                    version: if cur_version.is_empty() { "0.1.0".to_string() } else { cur_version.clone() },
                    pkg_type: cur_type.clone(),
                    tier: cur_tier.clone(),
                    is_meta: cur_meta,
                    description: cur_desc.clone(),
                    dependencies: cur_deps.clone(),
                    files: cur_files.clone(),
                });
            }
            cur_name.clear();
            cur_version.clear();
            cur_type = default_type.to_string();
            cur_tier = "optional".to_string();
            cur_meta = false;
            cur_desc.clear();
            cur_deps.clear();
            cur_files.clear();
        }
    }

    list
}

fn extract_array_items(s: &str, out: &mut Vec<String>) {
    let mut in_str = false;
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        if b == b'"' {
            if !in_str {
                in_str = true;
                start = i + 1;
            } else {
                in_str = false;
                if i >= start {
                    out.push(s[start..i].to_string());
                }
            }
        }
    }
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
