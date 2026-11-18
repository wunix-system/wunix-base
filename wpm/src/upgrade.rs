//! Модуль обновления прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{geteuid, getuid, println, t};
use libpackwunix::version::Version;

use crate::display::{confirm_prompt, format_size, print_access_denied, print_disk_space_change, print_error, print_step};
use crate::guard::ensure_wsp_system_safe;
use crate::install::{find_repo_package, load_installed_apps_from, save_installed_apps_to, InstalledApp};
use crate::search::load_wpm_catalog_from;
use crate::unpack::{read_file_to_vec, unpack_wpm_package_to};

struct UpgradableApp {
    name: String,
    old_version: Version,
    new_version: Version,
    size: u64,
    download_url: String,
    filename: String,
    sha256: String,
}

pub fn run_upgrade(
    root_prefix: Option<&str>,
    assume_yes: bool,
    ignore_unsafe: bool,
    verbose: bool,
) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("upgrade", uid);
        return 1;
    }

    if root_prefix.is_none() && ensure_wsp_system_safe(ignore_unsafe).is_err() {
        return 1;
    }

    let root_str = root_prefix.unwrap_or("/");
    println!("wpm: проверка доступных обновлений программ...");

    let installed = load_installed_apps_from(root_str);
    let catalog = load_wpm_catalog_from(root_str);

    let mut upgradable: Vec<UpgradableApp> = Vec::new();
    for app in &installed {
        let app_ver = Version::parse(&app.version).unwrap_or(Version::new(0, 1, 0));
        if let Some(cat_pkg) = catalog.iter().find(|p| p.name == app.name || p.aliases.iter().any(|a| a == &app.name)) {
            if let Ok(cat_ver) = Version::parse(&cat_pkg.version) {
                if cat_ver.is_newer_than(&app_ver) {
                    if let Some((repo_pkg, download_url)) = find_repo_package(&cat_pkg.name, root_prefix) {
                        upgradable.push(UpgradableApp {
                            name: cat_pkg.name.clone(),
                            old_version: app_ver,
                            new_version: cat_ver,
                            size: repo_pkg.size,
                            download_url,
                            filename: repo_pkg.filename,
                            sha256: repo_pkg.sha256,
                        });
                    }
                }
            }
        }
    }

    if upgradable.is_empty() {
        println!("wpm: все установленные программы имеют актуальные версии.");
        return 0;
    }

    println!("{}:", t("Программы, подготовленные к обновлению", "Packages to be upgraded"));
    let mut total_download: u64 = 0;
    let mut total_installed: u64 = 0;
    for item in &upgradable {
        total_download += item.size;
        total_installed += item.size.saturating_mul(2);
        println!(
            "  - {:<16} v{} -> v{} ({})",
            item.name,
            item.old_version,
            item.new_version,
            format_size(item.size)
        );
    }

    print_disk_space_change(total_download, total_installed, true);

    if !assume_yes {
        let prompt_text = t(
            "Применить обновление программ? [Y/n] ",
            "Apply application upgrades? [Y/n] ",
        );
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: обновление программ отменено пользователем.", "wpm: upgrade aborted by user."));
            return 0;
        }
    }

    let cache_dir = if let Some(r) = root_prefix {
        format!("{}/var/cache/wpm", r.trim_end_matches('/'))
    } else {
        "/var/cache/wpm".to_string()
    };
    crate::install::ensure_dir_recursive(&cache_dir);

    let mut new_installed = installed.clone();
    let total_steps = upgradable.len();
    let mut current_step = 1;

    for item in &upgradable {
        let step_label = format!("Обновление программы {}", item.name);
        print_step(current_step, total_steps, &step_label, true);
        current_step += 1;

        let dest_path = format!("{}/{}", cache_dir, item.filename);
        let pkg_data = match read_file_to_vec(&dest_path) {
            Ok(d) => d,
            Err(_) => {
                println!("wpm: загрузка пакета '{}' ({}) для обновления...", item.name, format_size(item.size));
                if let Err(e) = crate::http::fetch_to_file(&item.download_url, &dest_path, Some(&item.sha256), &item.filename) {
                    print_error(&format!("не удалось загрузить {}: {:?}", item.filename, e));
                    return 1;
                }
                match read_file_to_vec(&dest_path) {
                    Ok(d) => d,
                    Err(e) => {
                        print_error(e);
                        return 1;
                    }
                }
            }
        };

        let (manifest, files) = match unpack_wpm_package_to(&pkg_data, root_prefix, verbose) {
            Ok(res) => res,
            Err(e) => {
                print_error(e);
                return 1;
            }
        };

        new_installed.retain(|a| a.name != manifest.name);
        new_installed.push(InstalledApp {
            name: manifest.name.clone(),
            version: manifest.version.to_string(),
            description: manifest.description.clone(),
            files,
        });
        save_installed_apps_to(root_str, &new_installed);
    }

    println!("wpm: обновление программ успешно завершено.");
    0
}
