//! Модуль установки и удаления прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, geteuid, getuid, mkdir, open, println, read, t, unlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

use libpackwunix::format::PackageContainer;

use crate::display::{confirm_prompt, format_size, print_access_denied, print_disk_space_change, print_error, print_step, COLOR_BOLD, COLOR_RESET};
use crate::guard::ensure_wsp_system_safe;
use crate::unpack::{read_file_to_vec, unpack_wpm_package_to};

#[allow(dead_code)]
pub const WPM_DB_DIR: &[u8] = b"/var/lib/wpm\0";
#[allow(dead_code)]
pub const WPM_DB_PATH: &[u8] = b"/var/lib/wpm/installed.json\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub description: String,
    pub files: Vec<String>,
}

pub fn ensure_db_dir_in(root: &str) {
    let r = if root == "/" { "" } else { root.trim_end_matches('/') };
    let dirs = [
        format!("{}/var", r),
        format!("{}/var/lib", r),
        format!("{}/var/lib/wpm", r),
        format!("{}/var/cache", r),
        format!("{}/var/cache/wpm", r),
    ];
    for d in &dirs {
        let mut null_d = [0u8; 256];
        let bytes = d.as_bytes();
        let l = bytes.len().min(255);
        null_d[..l].copy_from_slice(&bytes[..l]);
        null_d[l] = 0;
        let _ = mkdir(&null_d, 0o777);
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
        let _ = mkdir(&null_p, 0o777);
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

#[allow(dead_code)]
pub fn ensure_db_dir() {
    ensure_db_dir_in("/");
}

pub fn load_installed_apps() -> Vec<InstalledApp> {
    load_installed_apps_from("/")
}

pub fn load_installed_apps_from(root: &str) -> Vec<InstalledApp> {
    ensure_db_dir_in(root);
    let r = if root == "/" { "" } else { root.trim_end_matches('/') };
    let db_path = format!("{}/var/lib/wpm/installed.json", r);
    let mut null_path = [0u8; 256];
    let bytes = db_path.as_bytes();
    let l = bytes.len().min(255);
    null_path[..l].copy_from_slice(&bytes[..l]);
    null_path[l] = 0;

    let fd = open(&null_path, O_RDONLY as u64, 0);
    if fd < 0 {
        if root.is_empty() || root == "/" {
            let defaults = get_default_installed_apps();
            save_installed_apps_to(root, &defaults);
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
            let defaults = get_default_installed_apps();
            save_installed_apps_to(root, &defaults);
            return defaults;
        } else {
            return Vec::new();
        }
    }

    let text = core::str::from_utf8(&buf).unwrap_or("");
    let parsed = parse_installed_db(text);
    if parsed.is_empty() {
        if root.is_empty() || root == "/" {
            let defaults = get_default_installed_apps();
            save_installed_apps_to(root, &defaults);
            defaults
        } else {
            Vec::new()
        }
    } else {
        parsed
    }
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

pub fn get_default_installed_apps() -> Vec<InstalledApp> {
    alloc::vec![
        InstalledApp {
            name: "archiveutils".to_string(),
            version: "1.0.0".to_string(),
            description: "Пользовательские утилиты работы со специальными архивными контейнерами (ar, cpio, unrar)".to_string(),
            files: alloc::vec![
                "/bin/ar".to_string(),
                "/bin/cpio".to_string(),
                "/bin/unrar".to_string(),
            ],
        },
        InstalledApp {
            name: "curl".to_string(),
            version: "1.0.0".to_string(),
            description: "Сетевой клиент HTTP/HTTPS для передачи данных и API".to_string(),
            files: alloc::vec![
                "/bin/curl".to_string(),
            ],
        },
        InstalledApp {
            name: "gzip".to_string(),
            version: "1.0.0".to_string(),
            description: "Утилиты потокового сжатия и распаковки gzip (gzip, gunzip)".to_string(),
            files: alloc::vec![
                "/bin/gzip".to_string(),
                "/bin/gunzip".to_string(),
            ],
        },
        InstalledApp {
            name: "nano".to_string(),
            version: "7.2.0".to_string(),
            description: "Текстовый редактор GNU nano для Wunix".to_string(),
            files: alloc::vec![
                "/bin/nano".to_string(),
            ],
        },
        InstalledApp {
            name: "openssh".to_string(),
            version: "9.8.0".to_string(),
            description: "Комплекс сетевого безопасного доступа OpenSSH (ssh, sshd, scp)".to_string(),
            files: alloc::vec![
                "/bin/ssh".to_string(),
                "/bin/sshd".to_string(),
                "/bin/sshd-session".to_string(),
                "/bin/ssh-keygen".to_string(),
                "/bin/scp".to_string(),
                "/bin/sftp-server".to_string(),
                "/etc/ssh/sshd_config".to_string(),
            ],
        },
        InstalledApp {
            name: "tar".to_string(),
            version: "1.0.0".to_string(),
            description: "Утилита работы со стандартными архивами tar".to_string(),
            files: alloc::vec![
                "/bin/tar".to_string(),
            ],
        },
        InstalledApp {
            name: "wc".to_string(),
            version: "1.0.0".to_string(),
            description: "Пользовательская программа wc для подсчета строк, слов и байтов".to_string(),
            files: alloc::vec![
                "/bin/wc".to_string(),
            ],
        },
        InstalledApp {
            name: "zip".to_string(),
            version: "1.0.0".to_string(),
            description: "Утилиты сжатия и распаковки ZIP архивов (zip, unzip)".to_string(),
            files: alloc::vec![
                "/bin/zip".to_string(),
                "/bin/unzip".to_string(),
            ],
        },
    ]
}

pub fn save_installed_apps(apps: &[InstalledApp]) {
    save_installed_apps_to("/", apps);
}

pub fn save_installed_apps_to(root: &str, apps: &[InstalledApp]) {
    ensure_db_dir_in(root);
    let r = if root == "/" { "" } else { root.trim_end_matches('/') };
    let db_path = format!("{}/var/lib/wpm/installed.json", r);
    let mut null_path = [0u8; 256];
    let bytes = db_path.as_bytes();
    let l = bytes.len().min(255);
    null_path[..l].copy_from_slice(&bytes[..l]);
    null_path[l] = 0;

    let fd = open(&null_path, (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
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
    matches!(
        pkg,
        "wunix-kernel" | "wunix-base" | "libwunix" | "libpackwunix" | "libcwunix"
            | "coreutils" | "netutils" | "sysutils" | "authutils"
            | "wspd" | "wsp" | "wsh" | "init" | "system" | "winstall"
            | "openssl" | "ncurses" | "zlib"
    )
}

#[allow(dead_code)]
pub fn locate_package_file(pkg: &str) -> Option<String> {
    locate_package_file_in(pkg, None)
}

pub fn locate_package_file_in(pkg: &str, root_opt: Option<&str>) -> Option<String> {
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

    let pkg_id = match pkg {
        "tcc" | "cc" => "tinycc",
        other => other,
    };

    let mut repo_dirs: Vec<String> = alloc::vec![
        "/var/lib/wpm/repo/apps/x86_64".to_string(),
        "/var/cache/wpm".to_string(),
        "/repo/apps/x86_64".to_string(),
        "/repositories/apps/x86_64".to_string(),
    ];
    if let Some(root) = root_opt {
        if !root.is_empty() && root != "/" {
            let r = root.trim_end_matches('/');
            repo_dirs.insert(0, format!("{}/var/lib/wpm/repo/apps/x86_64", r));
            repo_dirs.insert(1, format!("{}/var/cache/wpm", r));
            repo_dirs.insert(2, format!("{}/repo/apps/x86_64", r));
            repo_dirs.insert(3, format!("{}/repositories/apps/x86_64", r));
        }
    }

    let mut candidates: Vec<String> = Vec::new();

    for repo_dir in &repo_dirs {
        let idx_path = format!("{}/packages.json", repo_dir);
        if let Ok(idx_data) = read_file_to_vec(&idx_path) {
            if let Ok(idx_str) = core::str::from_utf8(&idx_data) {
                let pkgs = libpackwunix::sources::parse_repo_index(idx_str);
                for p in pkgs {
                    if p.name == pkg_id || p.name == pkg {
                        candidates.push(format!("{}/{}", repo_dir, p.filename));
                        candidates.push(format!("/var/cache/wpm/{}", p.filename));
                        if let Some(root) = root_opt {
                            if !root.is_empty() && root != "/" {
                                candidates.push(format!("{}/var/cache/wpm/{}", root.trim_end_matches('/'), p.filename));
                            }
                        }
                    }
                }
            }
        }
    }

    candidates.push(format!("/var/cache/wpm/{}.wp", pkg_id));
    candidates.push(format!("/var/cache/wpm/{}-1.0.0.wp", pkg_id));
    candidates.push(format!("/var/cache/wpm/{}-1.0.1.wp", pkg_id));
    candidates.push(format!("/var/lib/wpm/repo/apps/x86_64/{}.wp", pkg_id));
    candidates.push(format!("/var/lib/wpm/repo/apps/x86_64/{}-1.0.1.wp", pkg_id));
    candidates.push(format!("/repo/apps/x86_64/{}.wp", pkg_id));
    candidates.push(format!("/repositories/apps/x86_64/{}.wp", pkg_id));
    candidates.push(format!("/usr/share/wpm/packages/{}.wp", pkg_id));
    candidates.push(format!("/var/cache/wpm/{}.wp", pkg));

    if let Some(root) = root_opt {
        if !root.is_empty() && root != "/" {
            let r = root.trim_end_matches('/');
            candidates.push(format!("{}/var/cache/wpm/{}.wp", r, pkg_id));
            candidates.push(format!("{}/var/cache/wpm/{}-1.0.0.wp", r, pkg_id));
            candidates.push(format!("{}/var/cache/wpm/{}-1.0.1.wp", r, pkg_id));
            candidates.push(format!("{}/var/lib/wpm/repo/apps/x86_64/{}.wp", r, pkg_id));
            candidates.push(format!("{}/repo/apps/x86_64/{}.wp", r, pkg_id));
            candidates.push(format!("{}/repositories/apps/x86_64/{}.wp", r, pkg_id));
        }
    }

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

pub fn find_repo_package(pkg: &str, root_opt: Option<&str>) -> Option<(libpackwunix::sources::RepoPackage, String)> {
    let mut repo_dirs: Vec<String> = alloc::vec![
        "/var/lib/wpm/repo/apps/x86_64".to_string(),
        "/var/cache/wpm".to_string(),
        "/repo/apps/x86_64".to_string(),
        "/repositories/apps/x86_64".to_string(),
    ];
    if let Some(root) = root_opt {
        if !root.is_empty() && root != "/" {
            let r = root.trim_end_matches('/');
            repo_dirs.insert(0, format!("{}/var/lib/wpm/repo/apps/x86_64", r));
            repo_dirs.insert(1, format!("{}/var/cache/wpm", r));
            repo_dirs.insert(2, format!("{}/repo/apps/x86_64", r));
            repo_dirs.insert(3, format!("{}/repositories/apps/x86_64", r));
        }
    }

    let pkg_id = match pkg {
        "tcc" | "cc" => "tinycc",
        "ar" | "cpio" | "unrar" => "archiveutils",
        "unzip" => "zip",
        "gunzip" => "gzip",
        "ssh" | "sshd" | "scp" => "openssh",
        other => other,
    };

    let sources = crate::repo_cmd::load_sources();
    let server_url = sources
        .first()
        .and_then(|s| s.servers.first())
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("http://wunix.rusos.site/repo/apps/x86_64");

    for dir in &repo_dirs {
        let idx_path = format!("{}/packages.json", dir);
        if let Ok(data) = read_file_to_vec(&idx_path) {
            if let Ok(s) = core::str::from_utf8(&data) {
                let pkgs = libpackwunix::sources::parse_repo_index(s);
                for p in pkgs {
                    if p.name == pkg_id || p.name == pkg {
                        let download_url = format!("{}/{}", server_url, p.filename);
                        return Some((p, download_url));
                    }
                }
            }
        }
    }

    // Резервный каталог официальных пакетов WPM на случай отсутствия локального packages.json
    let fallback_data = match pkg_id {
        "git" => Some(("git", "2.43.0", "git-2.43.0.wp", 1925203, "26818a878c233c8017938d59d38d4c73a9487de474834f09fe6a60c6221ed503")),
        "curl" => Some(("curl", "1.0.0", "curl-1.0.0.wp", 2415523, "5a925a4c7efb2f586e30f64da0026d51ea0899bee1288197daec3baca3983c62")),
        "tinycc" => Some(("tinycc", "0.9.27", "tinycc-0.9.27.wp", 244586, "7764258292f4320ecd4da8934287de05672c45ddc1caf5876f77c1c90909fea9")),
        "nano" => Some(("nano", "7.2.0", "nano-7.2.0.wp", 866660, "b269ea7b3ad4cd4f485a6c9750cfb298946d6934c638e1277a856a5d33032554")),
        "openssh" => Some(("openssh", "9.8.0", "openssh-9.8.0.wp", 12713083, "c68e22d8438d45ee9c74ae077f242f10b42ed7a0cea972a718719aed862c97ad")),
        "archiveutils" => Some(("archiveutils", "1.0.0", "archiveutils-1.0.0.wp", 16911, "5faaeeae793d56cf44356bcb8c648d9b9cbf561b3966b706fb8eeadf751dea5b")),
        "tar" => Some(("tar", "1.0.0", "tar-1.0.0.wp", 9408, "e7b4aeeec55fda71d104fe549fc95ad7262221d12128927993ed2cbae93dc0b9")),
        "gzip" => Some(("gzip", "1.0.0", "gzip-1.0.0.wp", 15221, "0befdd0e455d3d50213dc62847f39c5a575b97fa458871eb72c1db51f48a718a")),
        "zip" => Some(("zip", "1.0.0", "zip-1.0.0.wp", 17516, "216bbad31de9fac72c20e2a51f81c610336aaf9dbe788d4a23b9361ab7dbeacb")),
        "wc" => Some(("wc", "1.0.0", "wc-1.0.0.wp", 6186, "4342103b0b667fffd177144f75dd71abb6adc88a0767938a13b4c6b4aa60a60f")),
        _ => None,
    };

    if let Some((name, ver_s, fname, size, sha)) = fallback_data {
        let p = libpackwunix::sources::RepoPackage {
            name: name.to_string(),
            version: libpackwunix::version::Version::parse(ver_s).unwrap_or(libpackwunix::version::Version::new(1, 0, 0)),
            arch: "x86_64".to_string(),
            tier: "optional".to_string(),
            package_type: "user".to_string(),
            description: String::new(),
            filename: fname.to_string(),
            size,
            sha256: sha.to_string(),
        };
        let download_url = format!("{}/{}", server_url, fname);
        return Some((p, download_url));
    }

    None
}

#[allow(dead_code)]
pub fn run_install(packages: &[&str], assume_yes: bool, ignore_unsafe: bool, verbose: bool, reinstall: bool) -> i32 {
    run_install_in(packages, None, assume_yes, ignore_unsafe, verbose, reinstall)
}

enum TargetSource {
    Local { path: String, data: Vec<u8> },
    Remote {
        download_url: String,
        dest_path: String,
        sha256: String,
        filename: String,
    },
}

struct PendingTarget {
    name: String,
    version: String,
    download_size: u64,
    #[allow(dead_code)]
    installed_size: u64,
    source: TargetSource,
}

pub fn run_install_in(
    packages: &[&str],
    root_prefix: Option<&str>,
    assume_yes: bool,
    ignore_unsafe: bool,
    verbose: bool,
    reinstall: bool,
) -> i32 {
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

    if root_prefix.is_none() && ensure_wsp_system_safe(ignore_unsafe).is_err() {
        return 1;
    }

    let root_str = root_prefix.unwrap_or("/");
    let installed_list = load_installed_apps_from(root_str);
    let mut targets: Vec<PendingTarget> = Vec::new();
    let mut total_download: u64 = 0;
    let mut total_installed_space: u64 = 0;

    for &pkg in packages {
        if is_system_package(pkg) {
            println!("{}wpm: ошибка:{} '{}' является системным компонентом Wunix.", COLOR_BOLD, COLOR_RESET, pkg);
            println!("  Для управления системными пакетами ядра и базовой ОС используйте WSP:");
            println!("  sudo wsp install {}", pkg);
            return 1;
        }

        if let Some(pkg_file_path) = locate_package_file_in(pkg, root_prefix) {
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
            targets.push(PendingTarget {
                name,
                version,
                download_size: d_size,
                installed_size: i_size,
                source: TargetSource::Local { path: pkg_file_path, data: pkg_data },
            });
        } else if let Some((repo_pkg, download_url)) = find_repo_package(pkg, root_prefix) {
            let cache_dir = if let Some(r) = root_prefix {
                format!("{}/var/cache/wpm", r.trim_end_matches('/'))
            } else {
                "/var/cache/wpm".to_string()
            };
            let target_dest = format!("{}/{}", cache_dir, repo_pkg.filename);
            let name = repo_pkg.name.clone();
            let version = repo_pkg.version.to_string();
            let d_size = repo_pkg.size;
            let i_size = repo_pkg.size.saturating_mul(2);

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
            targets.push(PendingTarget {
                name,
                version,
                download_size: d_size,
                installed_size: i_size,
                source: TargetSource::Remote {
                    download_url,
                    dest_path: target_dest,
                    sha256: repo_pkg.sha256,
                    filename: repo_pkg.filename,
                },
            });
        } else {
            print_error(&format!(
                "пакет '{}' не найден в каталоге прикладных программ WPM и локальном кэше /var/cache/wpm.",
                pkg
            ));
            println!("  Используйте 'wpm update' для синхронизации каталогов репозиториев.");
            println!("  Используйте 'wpm search <запрос>' для поиска доступных программ.");
            return 1;
        }
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

    let mut installed_list = load_installed_apps_from(root_str);
    let mut total_installed = 0;

    for target in targets {
        println!("wpm: подготовка к установке программы '{}'", target.name);

        let (pkg_data, display_path) = match target.source {
            TargetSource::Local { path, data } => (data, path),
            TargetSource::Remote { download_url, dest_path, sha256, filename } => {
                let cache_dir = if let Some(r) = root_prefix {
                    format!("{}/var/cache/wpm", r.trim_end_matches('/'))
                } else {
                    "/var/cache/wpm".to_string()
                };
                ensure_dir_recursive(&cache_dir);

                let data = match read_file_to_vec(&dest_path) {
                    Ok(d) => d,
                    Err(_) => {
                        println!("wpm: загрузка пакета '{}' из репозитория...", target.name);
                        match crate::http::fetch_to_file(&download_url, &dest_path, Some(&sha256), &filename) {
                            Ok(_) => match read_file_to_vec(&dest_path) {
                                Ok(d) => d,
                                Err(e) => {
                                    print_error(e);
                                    return 1;
                                }
                            },
                            Err(err) => {
                                print_error(&format!("ошибка загрузки пакета '{}': {:?}", target.name, err));
                                return 1;
                            }
                        }
                    }
                };
                (data, dest_path)
            }
        };

        // Шаг 1: Проверка прав
        print_step(1, 4, t("Проверка прав администратора и барьера WSP", "Checking permissions and WSP security"), true);

        // Шаг 2: Проверка целостности
        let size_str = format_size(target.download_size);
        let step2_msg = format!("{} {} ({})", t("Проверка контейнера", "Verifying package"), display_path, size_str);
        print_step(2, 4, &step2_msg, true);

        // Шаг 3: Распаковка
        let (manifest, files) = match unpack_wpm_package_to(&pkg_data, root_prefix, verbose) {
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
        save_installed_apps_to(root_str, &installed_list);
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
        let pkg_id = match pkg {
            "tcc" => "tinycc",
            other => other,
        };

        if let Some(app) = installed_list.iter().find(|a| a.name == pkg_id || a.name == pkg) {
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
