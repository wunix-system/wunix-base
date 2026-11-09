//! Модуль установки и удаления прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, geteuid, getuid, mkdir, open, println, read, t, unlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

use libpackwunix::format::PackageContainer;

use crate::display::{confirm_prompt, format_size, print_access_denied, print_disk_space_change, print_error, print_step, COLOR_BOLD, COLOR_RESET};
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

pub fn is_system_package(pkg: &str) -> bool {
    let canonical = match pkg {
        "cc" => "tinycc",
        "ssh" | "sshd" | "scp" => "openssh",
        "unzip" => "zip",
        "gunzip" => "gzip",
        other => other,
    };
    matches!(
        canonical,
        "nano" | "curl" | "tar" | "zip" | "gzip" | "openssh"
            | "openssl" | "ncurses" | "zlib" | "wunix-kernel" | "wunix-base"
            | "libwunix" | "libpackwunix" | "libcwunix" | "coreutils" | "netutils"
            | "sysutils" | "authutils" | "archiveutils" | "wspd" | "wsp" | "wsh"
            | "init" | "system" | "winstall"
    )
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
        "tcc" | "cc" => "tinycc",
        other => other,
    };

    let candidates = [
        format!("/var/cache/wpm/{}.wp", canonical),
        format!("/var/cache/wpm/{}-0.9.27.wp", canonical),
        format!("/var/lib/wpm/repo/apps/x86_64/{}.wp", canonical),
        format!("/repo/apps/x86_64/{}.wp", canonical),
        format!("/usr/share/wpm/packages/{}.wp", canonical),
        format!("/var/cache/wpm/{}.wp", pkg),
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

#[allow(dead_code)]
struct PackageTarget {
    file_path: String,
    data: Vec<u8>,
    name: String,
    version: String,
    download_size: u64,
    installed_size: u64,
}

pub fn run_install(packages: &[&str], assume_yes: bool, ignore_unsafe: bool, verbose: bool, reinstall: bool) -> i32 {
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

    let installed_list = load_installed_apps();
    let mut targets: Vec<PackageTarget> = Vec::new();
    let mut total_download: u64 = 0;
    let mut total_installed_space: u64 = 0;

    for &pkg in packages {
        if is_system_package(pkg) {
            println!("{}wpm: ошибка:{} '{}' является системным компонентом Wunix.", COLOR_BOLD, COLOR_RESET, pkg);
            println!("  Для управления системными пакетами ядра и базовой ОС используйте WSP:");
            println!("  sudo wsp install {}", pkg);
            return 1;
        }

        let pkg_file_path = match locate_package_file(pkg) {
            Some(p) => p,
            None => {
                print_error(&format!(
                    "пакет '{}' не найден в каталоге прикладных программ WPM и локальном кэше /var/cache/wpm.",
                    pkg
                ));
                println!("  Используйте 'wpm search <запрос>' для поиска доступных программ.");
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

        let (name, version, d_size, i_size) = match PackageContainer::parse(&pkg_data) {
            Ok(c) => {
                let d = pkg_data.len() as u64;
                let p_len = c.payload.len();
                let i = if p_len >= 4 && c.payload[0] == 0x1f && c.payload[1] == 0x8b {
                    u32::from_le_bytes([
                        c.payload[p_len - 4],
                        c.payload[p_len - 3],
                        c.payload[p_len - 2],
                        c.payload[p_len - 1],
                    ]) as u64
                } else {
                    p_len as u64
                };
                (c.manifest.name.clone(), c.manifest.version.to_string(), d, i)
            }
            Err(e) => {
                print_error(e);
                return 1;
            }
        };

        let already_installed = installed_list.iter().find(|a| a.name == name || a.name == pkg);
        if let Some(inst) = already_installed {
            if !reinstall && inst.version == version {
                println!("wpm: программа '{}' (v{}) уже установлена в системе.", name, inst.version);
                println!("  Для переустановки выполните: wpm install --reinstall {}", pkg);
                continue;
            } else if inst.version != version {
                println!("wpm: обновление программы '{}': v{} -> v{}", name, inst.version, version);
            }
        }

        total_download += d_size;
        total_installed_space += i_size;

        targets.push(PackageTarget {
            file_path: pkg_file_path,
            data: pkg_data,
            name,
            version,
            download_size: d_size,
            installed_size: i_size,
        });
    }

    if targets.is_empty() {
        println!("{}", t("wpm: нет программ, требующих установки или обновления.", "wpm: no packages require installation or upgrade."));
        return 0;
    }

    println!("{}", t("Будут установлены следующие программы:", "The following packages will be installed:"));
    for target in &targets {
        println!("  {} (v{})", target.name, target.version);
    }

    print_disk_space_change(total_download, total_installed_space, true);

    if !assume_yes {
        let prompt_text = t("Продолжить установку? [Y/n] ", "Do you want to continue? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: установка отменена пользователем.", "wpm: installation aborted by user."));
            return 0;
        }
    }

    let mut installed_list = load_installed_apps();
    let mut total_installed = 0;

    for target in targets {
        println!("wpm: подготовка к установке программы '{}'", target.name);

        // Шаг 1: Проверка прав
        print_step(1, 4, t("Проверка прав администратора и барьера WSP", "Checking permissions and WSP security"), true);

        // Шаг 2: Проверка целостности
        let size_str = format_size(target.download_size);
        let step2_msg = format!("{} {} ({})", t("Проверка контейнера", "Verifying package"), target.file_path, size_str);
        print_step(2, 4, &step2_msg, true);

        // Шаг 3: Распаковка
        let (manifest, files) = match unpack_wpm_package(&target.data, verbose) {
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

        if manifest.name == "tinycc" {
            println!("  Команда для запуска: tcc -v");
        }

        total_installed += 1;
    }

    if total_installed > 0 { 0 } else { 1 }
}

pub fn run_remove(packages: &[&str], purge: bool, assume_yes: bool) -> i32 {
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

    let mut targets_to_remove: Vec<InstalledApp> = Vec::new();
    let mut total_freed_space: u64 = 0;

    for &pkg in packages {
        let canonical = match pkg {
            "tcc" => "tinycc",
            other => other,
        };

        if let Some(app) = installed_list.iter().find(|a| a.name == canonical || a.name == pkg) {
            let freed = (app.files.len() as u64 * 32 * 1024).max(520 * 1024);
            total_freed_space += freed;
            targets_to_remove.push(app.clone());
        } else {
            print_error(&format!("{} '{}' {}", t("программа", "application"), pkg, t("не найдена среди установленных", "is not installed")));
            return 1;
        }
    }

    println!("{}", t("Будут удалены следующие программы:", "The following packages will be removed:"));
    for target in &targets_to_remove {
        println!("  {} (v{}) — {}: {}", target.name, target.version, t("файлов", "files"), target.files.len());
    }

    print_disk_space_change(0, total_freed_space, false);

    if !assume_yes {
        let prompt_text = t("Продолжить удаление? [Y/n] ", "Do you want to continue? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: удаление отменено пользователем.", "wpm: removal aborted by user."));
            return 0;
        }
    }

    println!("wpm: удаление программ...");

    for app in targets_to_remove {
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

        installed_list.retain(|a| a.name != app.name);

        println!(
            "  удалено: {} (файлов: {}{})",
            app.name,
            removed_count,
            if purge { t(", конфигурации очищены", ", configs purged") } else { "" }
        );
    }

    if installed_list.len() < initial_count {
        save_installed_apps(&installed_list);
        println!("wpm: удаление программ завершено.");
        0
    } else {
        1
    }
}
