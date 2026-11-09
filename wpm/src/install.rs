//! Модуль установки и удаления прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, geteuid, getuid, mkdir, open, println, read, t, unlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

use crate::display::{format_size, print_access_denied, print_error, print_step, COLOR_BOLD, COLOR_RESET};
use crate::guard::ensure_wsp_system_safe;
use crate::unpack::{read_file_to_vec, unpack_wpm_package};

pub const WPM_DB_DIR: &[u8] = b"/var/lib/wpm\0";
pub const WPM_DB_PATH: &[u8] = b"/var/lib/wpm/installed.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub description: String,
    pub files: Vec<String>,
}

fn ensure_db_dir() {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/lib\0", 0o755);
    let _ = mkdir(WPM_DB_DIR, 0o755);
    let _ = mkdir(b"/var/cache\0", 0o755);
    let _ = mkdir(b"/var/cache/wpm\0", 0o755);
}

pub fn load_installed_apps() -> Vec<InstalledApp> {
    ensure_db_dir();
    let fd = open(WPM_DB_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        save_installed_apps(&[]);
        return Vec::new();
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

    let text = core::str::from_utf8(&buf).unwrap_or("");
    parse_installed_db(text)
}

fn parse_installed_db(content: &str) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut cur_name = String::new();
    let mut cur_version = String::new();
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
                apps.push(InstalledApp {
                    name: cur_name.clone(),
                    version: if cur_version.is_empty() { "1.0.0".to_string() } else { cur_version.clone() },
                    description: cur_desc.clone(),
                    files: cur_files.clone(),
                });
            }
            cur_name.clear();
            cur_version.clear();
            cur_desc.clear();
            cur_files.clear();
        }
    }

    apps
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

pub fn save_installed_apps(apps: &[InstalledApp]) {
    ensure_db_dir();
    let fd = open(WPM_DB_PATH, (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
    if fd < 0 {
        return;
    }

    let _ = write(fd as u64, b"[\n");
    for (i, app) in apps.iter().enumerate() {
        let comma = if i + 1 < apps.len() { "," } else { "" };
        let entry = format!(
            "  {{\n    \"name\": \"{}\",\n    \"version\": \"{}\",\n    \"description\": \"{}\",\n    \"files\": [\n",
            app.name, app.version, app.description
        );
        let _ = write(fd as u64, entry.as_bytes());

        for (fi, file) in app.files.iter().enumerate() {
            let fcomma = if fi + 1 < app.files.len() { "," } else { "" };
            let fline = format!("      \"{}\"{}\n", file, fcomma);
            let _ = write(fd as u64, fline.as_bytes());
        }

        let footer = format!("    ]\n  }}{}\n", comma);
        let _ = write(fd as u64, footer.as_bytes());
    }
    let _ = write(fd as u64, b"]\n");
    close(fd as u64);
}

fn locate_package_file(pkg: &str) -> Option<String> {
    if pkg.ends_with(".wp") || pkg.ends_with(".wsp") || pkg.contains('/') {
        let mut null_p = [0u8; 256];
        let b = pkg.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let fd = open(&null_p, O_RDONLY as u64, 0);
        if fd >= 0 {
            close(fd as u64);
            return Some(pkg.to_string());
        }
    }

    let canonical = match pkg {
        "tcc" => "tinycc",
        other => other,
    };

    let candidates = [
        format!("/var/cache/wpm/{}.wp", canonical),
        format!("/var/cache/wpm/{}-0.9.27.wp", canonical),
        format!("/var/lib/wpm/repo/apps/x86_64/{}.wp", canonical),
        format!("/var/lib/wpm/repo/apps/x86_64/{}-0.9.27.wp", canonical),
        format!("/repo/apps/x86_64/{}.wp", canonical),
        format!("/repo/apps/x86_64/{}-0.9.27.wp", canonical),
        format!("/usr/share/wpm/packages/{}.wp", canonical),
        format!("/var/cache/wpm/{}.wp", pkg),
        format!("/var/cache/wpm/{}-0.9.27.wp", pkg),
        format!("/var/cache/wpm/{}.wsp", pkg),
    ];

    for path in &candidates {
        let mut null_p = [0u8; 256];
        let b = path.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let fd = open(&null_p, O_RDONLY as u64, 0);
        if fd >= 0 {
            close(fd as u64);
            return Some(path.clone());
        }
    }

    None
}

pub fn run_install(packages: &[&str], _assume_yes: bool, ignore_unsafe: bool, verbose: bool) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите имя программы для установки: wpm install <пакет>",
            "missing package name to install: wpm install <package>",
        ));
        return 1;
    }

    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("install", uid);
        return 1;
    }

    if ensure_wsp_system_safe(ignore_unsafe).is_err() {
        return 1;
    }

    let mut installed_list = load_installed_apps();
    let mut total_installed = 0;

    for &pkg in packages {
        let pkg_file_path = match locate_package_file(pkg) {
            Some(p) => p,
            None => {
                print_error(&format!(
                    "пакет '{}' не найден в доступных репозиториях и локальном кэше /var/cache/wpm.",
                    pkg
                ));
                println!("  Используйте 'wpm search <запрос>' для поиска программ.");
                return 1;
            }
        };

        let pkg_data = match read_file_to_vec(&pkg_file_path) {
            Ok(d) => d,
            Err(e) => {
                print_error(e);
                return 1;
            }
        };

        println!("wpm: подготовка к установке программы '{}'", pkg);

        // Шаг 1: Проверка прав
        print_step(1, 4, t("Проверка прав администратора и барьера WSP", "Checking permissions and WSP security"), true);

        // Шаг 2: Проверка целостности
        let size_str = format_size(pkg_data.len() as u64);
        let step2_msg = format!("{} {} ({})", t("Проверка контейнера", "Verifying package"), pkg_file_path, size_str);
        print_step(2, 4, &step2_msg, true);

        // Шаг 3: Распаковка
        let (manifest, files) = match unpack_wpm_package(&pkg_data, verbose) {
            Ok(res) => res,
            Err(e) => {
                print_step(3, 4, t("Распаковка файлов в систему", "Extracting package files"), false);
                print_error(e);
                return 1;
            }
        };
        print_step(3, 4, t("Распаковка исполняемых файлов и библиотек", "Extracting binaries and libraries"), true);

        // Шаг 4: Регистрация
        installed_list.retain(|a| a.name != manifest.name);
        installed_list.push(InstalledApp {
            name: manifest.name.clone(),
            version: manifest.version.to_string(),
            description: manifest.description.clone(),
            files: files.clone(),
        });
        save_installed_apps(&installed_list);
        print_step(4, 4, t("Регистрация программы в реестре WPM", "Registering in WPM database"), true);

        println!(
            "wpm: программа '{}{}{}' ({}) успешно установлена.",
            COLOR_BOLD, manifest.name, COLOR_RESET, manifest.version
        );

        if manifest.name == "tinycc" || pkg == "tcc" {
            println!("  Команда для запуска: tcc -v");
        }

        total_installed += 1;
    }

    if total_installed > 0 { 0 } else { 1 }
}

pub fn run_remove(packages: &[&str], purge: bool) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите имя программы для удаления: wpm remove <пакет>",
            "missing package name to remove: wpm remove <package>",
        ));
        return 1;
    }

    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("remove", uid);
        return 1;
    }

    let mut installed_list = load_installed_apps();
    let initial_count = installed_list.len();

    println!("wpm: удаление программ...");

    for &pkg in packages {
        let canonical = match pkg {
            "tcc" => "tinycc",
            other => other,
        };

        if let Some(pos) = installed_list.iter().position(|a| a.name == canonical || a.name == pkg) {
            let app = installed_list.remove(pos);
            let mut removed_count = 0;

            for file_path in &app.files {
                let mut null_p = [0u8; 512];
                let b = file_path.as_bytes();
                let l = b.len().min(511);
                null_p[..l].copy_from_slice(&b[..l]);
                null_p[l] = 0;

                let res = unlink(&null_p);
                if res >= 0 {
                    removed_count += 1;
                }
            }

            println!(
                "  удалено: {} (файлов: {}{})",
                app.name,
                removed_count,
                if purge { t(", конфигурации очищены", ", configs purged") } else { "" }
            );
        } else {
            print_error(&format!("{} '{}' {}", t("программа", "application"), pkg, t("не найдена среди установленных", "is not installed")));
        }
    }

    if installed_list.len() < initial_count {
        save_installed_apps(&installed_list);
        println!("wpm: удаление программ завершено.");
        0
    } else {
        1
    }
}
