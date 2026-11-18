//! Модуль установки системных компонентов WSP.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{geteuid, getuid, println, t};
use libpackwunix::sources::RepoPackage;

use crate::db::{self, load_installed_packages_from, load_repo_packages, save_installed_packages_to, SystemPackageEntry};
use crate::display::{confirm_prompt, print_access_denied, print_disk_space_change, print_error, print_step};
use crate::http;
use crate::unpack::{self, read_file_to_vec};

pub fn locate_system_package(pkg: &str, root_dir: &str) -> Option<String> {
    if pkg.ends_with(".wp") || pkg.ends_with(".wsp") || pkg.contains('/') {
        if let Ok(d) = read_file_to_vec(pkg) {
            if libpackwunix::format::PackageContainer::parse(&d).is_ok() {
                return Some(pkg.to_string());
            }
        }
    }

    let mut candidates = alloc::vec![
        format!("/var/cache/wsp/{}.wp", pkg),
        format!("/var/cache/wsp/{}-1.0.0.wp", pkg),
        format!("/var/cache/wsp/{}-1.0.1.wp", pkg),
        format!("/var/cache/wsp/{}-1.0.2.wp", pkg),
        format!("/var/lib/wsp/repo/system/x86_64/{}.wp", pkg),
        format!("/var/lib/wsp/repo/system/x86_64/{}-1.0.0.wp", pkg),
        format!("/var/lib/wsp/repo/system/x86_64/{}-1.0.1.wp", pkg),
        format!("/var/lib/wsp/repo/system/x86_64/{}-1.0.2.wp", pkg),
        format!("/repositories/system/x86_64/{}.wp", pkg),
        format!("/repositories/system/x86_64/{}-1.0.0.wp", pkg),
        format!("/repositories/system/x86_64/{}-1.0.1.wp", pkg),
        format!("/repositories/system/x86_64/{}-1.0.2.wp", pkg),
    ];

    if !root_dir.is_empty() && root_dir != "/" {
        let r = root_dir.trim_end_matches('/');
        candidates.push(format!("{}/var/cache/wsp/{}.wp", r, pkg));
        candidates.push(format!("{}/var/cache/wsp/{}-1.0.0.wp", r, pkg));
        candidates.push(format!("{}/var/cache/wsp/{}-1.0.1.wp", r, pkg));
        candidates.push(format!("{}/var/cache/wsp/{}-1.0.2.wp", r, pkg));
        candidates.push(format!("{}/var/lib/wsp/repo/system/x86_64/{}.wp", r, pkg));
        candidates.push(format!("{}/var/lib/wsp/repo/system/x86_64/{}-1.0.1.wp", r, pkg));
        candidates.push(format!("{}/var/lib/wsp/repo/system/x86_64/{}-1.0.2.wp", r, pkg));
    }

    let repo_pkgs = load_repo_packages(root_dir);
    if let Some(rp) = repo_pkgs.iter().find(|p| p.name == pkg) {
        candidates.push(format!("/var/cache/wsp/{}", rp.filename));
        candidates.push(format!("/var/lib/wsp/repo/system/x86_64/{}", rp.filename));
        candidates.push(format!("/repositories/system/x86_64/{}", rp.filename));
        if !root_dir.is_empty() && root_dir != "/" {
            let r = root_dir.trim_end_matches('/');
            candidates.push(format!("{}/var/cache/wsp/{}", r, rp.filename));
        }
    }

    for path in &candidates {
        if let Ok(d) = read_file_to_vec(path) {
            if libpackwunix::format::PackageContainer::parse(&d).is_ok() {
                return Some(path.clone());
            }
        }
    }

    None
}

enum TargetSource {
    Local { path: String, data: Vec<u8> },
    Remote(RepoPackage),
}

struct PendingSystemTarget {
    name: String,
    version: String,
    tier: String,
    #[allow(dead_code)]
    download_size: u64,
    #[allow(dead_code)]
    installed_size: u64,
    source: TargetSource,
}

pub fn run_install(
    targets: &[&str],
    root_dir: &str,
    flag_yes: bool,
    flag_reinstall: bool,
    flag_verify: bool,
) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("install", uid);
        return 1;
    }

    if targets.is_empty() {
        print_error(t("укажите имя пакета для установки: wsp install <пакет>", "missing package: wsp install <package>"));
        return 1;
    }

    let installed_pkgs = load_installed_packages_from(root_dir);
    let repo_pkgs = load_repo_packages(root_dir);
    let mut pending: Vec<PendingSystemTarget> = Vec::new();
    let mut total_download: u64 = 0;
    let mut total_installed: u64 = 0;

    for &target in targets {
        if let Some(pkg_path) = locate_system_package(target, root_dir) {
            let pkg_data = match read_file_to_vec(&pkg_path) {
                Ok(d) => d,
                Err(e) => {
                    print_error(e);
                    return 1;
                }
            };

            let parsed = match libpackwunix::format::PackageContainer::parse(&pkg_data) {
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
                    Ok((c.manifest.name.clone(), c.manifest.version.to_string(), format!("{:?}", c.manifest.tier), d, i))
                }
                Err(e) => {
                    print_error(e);
                    Err(1)
                }
            };

            match parsed {
                Ok((name, version, tier, d_size, i_size)) => {
                    let already = installed_pkgs.iter().find(|p| p.name == name);
                    if let Some(inst) = already {
                        if !flag_reinstall && inst.version == version {
                            println!("wsp: системный компонент '{}' (v{}) уже установлен в системе.", name, inst.version);
                            println!("  Для принудительной переустановки выполните: wsp install --reinstall {}", pkg_path);
                            continue;
                        } else if inst.version != version {
                            println!("wsp: найдено обновление системного компонента '{}': v{} -> v{}", name, inst.version, version);
                        }
                    }

                    total_download += d_size;
                    total_installed += i_size;
                    pending.push(PendingSystemTarget {
                        name,
                        version,
                        tier,
                        download_size: d_size,
                        installed_size: i_size,
                        source: TargetSource::Local { path: pkg_path, data: pkg_data },
                    });
                }
                Err(code) => return code,
            }
        } else if let Some(rp) = repo_pkgs.iter().find(|p| p.name == target) {
            let name = rp.name.clone();
            let version = rp.version.to_string();
            let tier = rp.tier.clone();
            let d_size = rp.size;
            let i_size = rp.size.saturating_mul(2);

            let already = installed_pkgs.iter().find(|p| p.name == name);
            if let Some(inst) = already {
                if !flag_reinstall && inst.version == version {
                    println!("wsp: системный компонент '{}' (v{}) уже установлен в системе.", name, inst.version);
                    println!("  Для принудительной переустановки выполните: wsp install --reinstall {}", target);
                    continue;
                } else if inst.version != version {
                    println!("wsp: найдено обновление системного компонента '{}': v{} -> v{}", name, inst.version, version);
                }
            }

            total_download += d_size;
            total_installed += i_size;
            pending.push(PendingSystemTarget {
                name,
                version,
                tier,
                download_size: d_size,
                installed_size: i_size,
                source: TargetSource::Remote(rp.clone()),
            });
        } else {
            print_error(&format!("системный пакет '{}' не найден в кэше и репозитории WSP.", target));
            println!("  Используйте 'wsp list' или 'wsp check' для просмотра доступных компонентов.");
            return 1;
        }
    }

    if pending.is_empty() {
        println!("{}", t("wsp: нет системных компонентов, требующих установки.", "wsp: no components require installation."));
        return 0;
    }

    println!("{}:", t("Будет установлен системный компонент", "The following system component will be installed"));
    for item in &pending {
        println!("  {} (v{}) [{}]", item.name, item.version, item.tier);
    }

    print_disk_space_change(total_download, total_installed, true);

    if !flag_yes {
        let prompt_text = t("Продолжить установку системных компонентов? [Y/n] ", "Continue with installation? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wsp: установка отменена пользователем.", "wsp: installation aborted by user."));
            return 0;
        }
    }

    let mut status_code = 0;
    for item in pending {
        let (pkg_data, pkg_path) = match item.source {
            TargetSource::Local { path, data } => (data, path),
            TargetSource::Remote(rp) => {
                let base_url = libpackwunix::sources::get_wsp_system_repo().trim_end_matches('/');
                let download_url = format!("{}/{}", base_url, rp.filename);
                let cache_dir = format!("{}/var/cache/wsp", if root_dir.is_empty() { "" } else { root_dir.trim_end_matches('/') });
                db::ensure_dir_recursive(&cache_dir);
                let dest = format!("{}/{}", cache_dir, rp.filename);
                println!("wsp: загрузка системного пакета '{}' из репозитория...", rp.name);
                match http::fetch_to_file(&download_url, &dest, Some(&rp.sha256), &rp.filename) {
                    Ok(_) => match read_file_to_vec(&dest) {
                        Ok(d) => (d, dest),
                        Err(e) => {
                            print_error(e);
                            return 1;
                        }
                    },
                    Err(e) => {
                        print_error(&format!("не удалось загрузить {}: {:?}", rp.filename, e));
                        return 1;
                    }
                }
            }
        };

        let res = install_wsp_component(&pkg_data, &pkg_path, root_dir, flag_verify);
        if res != 0 {
            status_code = res;
        }
    }

    status_code
}

pub fn install_wsp_component(data: &[u8], pkg_file: &str, root_dir: &str, flag_verify: bool) -> i32 {
    println!("wsp: установка системного пакета '{}'", pkg_file);
    match unpack::unpack_wsp_package_to(data, root_dir, flag_verify) {
        Ok((manifest, files)) => {
            print_step(1, 2, t("Проверка сигнатуры и манифеста", "Verifying signature"), true);
            print_step(2, 2, t("Установка системных файлов", "Installing system files"), true);

            let mut installed = load_installed_packages_from(root_dir);
            installed.retain(|p| p.name != manifest.name);
            installed.push(SystemPackageEntry {
                name: manifest.name.clone(),
                version: manifest.version.to_string(),
                tier: format!("{:?}", manifest.tier),
                description: manifest.description.clone(),
                files: files.clone(),
            });
            save_installed_packages_to(root_dir, &installed);

            println!("wsp: системный компонент '{}' успешно установлен (файлов: {}).", manifest.name, files.len());
            0
        }
        Err(e) => {
            print_error(e);
            1
        }
    }
}
