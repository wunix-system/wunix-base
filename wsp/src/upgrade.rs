//! Модуль применения обновлений базовой системы WSP.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, geteuid, getuid, open, println, t, unlink, write, O_CREAT, O_TRUNC, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::{
    load_installed_packages_from, load_repo_packages,
    log_transaction, save_installed_packages_to, SystemPackageEntry, WSP_ALERT_PATH,
};
use crate::display::{
    confirm_prompt, format_size, print_access_denied, print_disk_space_change,
    print_step, COLOR_GREEN, COLOR_RED, COLOR_RESET,
};
use crate::unpack::{read_file_to_vec, unpack_wsp_package_to};

struct UpgradableItem {
    name: String,
    old_version: Version,
    new_version: Version,
    tier: String,
    size: u64,
    filename: String,
}

pub fn run_upgrade(root_dir: &str, critical_only: bool, assume_yes: bool, dry_run: bool) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("upgrade", uid);
        return 1;
    }

    println!("wsp: обновление базовой системы и ядра Wunix...");

    let installed = load_installed_packages_from(root_dir);
    let repo_packages = load_repo_packages(root_dir);

    let mut upgradable: Vec<UpgradableItem> = Vec::new();
    for inst in &installed {
        let inst_ver = Version::parse(&inst.version).unwrap_or(Version::new(1, 0, 0));
        if let Some(repo_pkg) = repo_packages.iter().find(|p| p.name == inst.name) {
            if repo_pkg.version > inst_ver {
                if critical_only && repo_pkg.tier != "critical" && repo_pkg.name != "wunix-kernel" {
                    continue;
                }
                upgradable.push(UpgradableItem {
                    name: inst.name.clone(),
                    old_version: inst_ver,
                    new_version: repo_pkg.version,
                    tier: repo_pkg.tier.clone(),
                    size: repo_pkg.size,
                    filename: repo_pkg.filename.clone(),
                });
            }
        }
    }

    if upgradable.is_empty() {
        if critical_only {
            println!("wsp: все критические компоненты ядра находятся в актуальном состоянии.");
        } else {
            println!("wsp: все системные компоненты находятся в актуальном состоянии.");
        }
        return 0;
    }

    println!("  Компоненты базы, подготовленные к обновлению:");
    let mut total_download: u64 = 0;
    let mut total_installed: u64 = 0;
    for item in &upgradable {
        total_download += item.size;
        total_installed += item.size.saturating_mul(2);
        let tier_label = if item.tier == "critical" {
            format!("{}[критический]{}", COLOR_RED, COLOR_RESET)
        } else {
            format!("{}[базовый]{}", COLOR_GREEN, COLOR_RESET)
        };
        println!(
            "    - {:<16} {} -> {}  {} {}",
            item.name,
            item.old_version,
            item.new_version,
            tier_label,
            format_size(item.size)
        );
    }

    print_disk_space_change(total_download, total_installed, true);

    if dry_run {
        println!("  Режим симуляции (--dry-run). Файлы ядра не изменялись.");
        return 0;
    }

    if !assume_yes {
        let prompt_text = t(
            "Применить системное обновление ядра? [Y/n] ",
            "Apply system kernel upgrade? [Y/n] ",
        );
        if !confirm_prompt(prompt_text, true) {
            println!(
                "{}",
                t(
                    "wsp: системное обновление отменено пользователем.",
                    "wsp: upgrade aborted by user."
                )
            );
            return 0;
        }
    }

    let total_steps = upgradable.len() + 1;
    let mut current_step = 1;
    let mut new_installed = installed.clone();

    for item in &upgradable {
        let step_label = format!("Обновление компонента {}", item.name);
        print_step(current_step, total_steps, &step_label, true);
        current_step += 1;

        let candidate_paths = [
            format!("/var/cache/wsp/{}", item.filename),
            format!("/var/lib/wsp/repo/system/x86_64/{}", item.filename),
            format!("/var/cache/wsp/{}.wp", item.name),
            format!("/var/cache/wsp/{}-{}.wp", item.name, item.new_version),
            format!("/var/lib/wsp/repo/system/x86_64/{}.wp", item.name),
            format!(
                "/var/lib/wsp/repo/system/x86_64/{}-{}.wp",
                item.name, item.new_version
            ),
        ];

        let mut pkg_data = None;
        for cp in &candidate_paths {
            if let Ok(d) = read_file_to_vec(cp) {
                pkg_data = Some(d);
                break;
            }
        }

        if pkg_data.is_none() {
            let base_url = libpackwunix::sources::get_wsp_system_repo().trim_end_matches('/');
            let download_url = format!("{}/{}", base_url, item.filename);
            let r = if root_dir == "/" { "" } else { root_dir.trim_end_matches('/') };
            let cache_dir = format!("{}/var/cache/wsp", r);
            crate::db::ensure_dir_recursive(&cache_dir);
            let dest_path = format!("{}/{}", cache_dir, item.filename);
            println!("wsp: загрузка компонента '{}' ({}) для обновления...", item.name, format_size(item.size));
            if crate::http::fetch_to_file(&download_url, &dest_path, None, &item.filename).is_ok() {
                if let Ok(d) = read_file_to_vec(&dest_path) {
                    pkg_data = Some(d);
                }
            }
        }

        if let Some(data) = pkg_data {
            if let Ok((manifest, files)) = unpack_wsp_package_to(&data, root_dir, false) {
                if item.name == "wunix-kernel" {
                    apply_kernel_rotation_with(
                        &item.new_version,
                        &item.old_version,
                        root_dir,
                    );
                }
                new_installed.retain(|p| p.name != item.name);
                new_installed.push(SystemPackageEntry {
                    name: item.name.clone(),
                    version: item.new_version.to_string(),
                    tier: item.tier.clone(),
                    description: manifest.description.clone(),
                    files,
                });
            }
        }
    }

    save_installed_packages_to(root_dir, &new_installed);
    let _ = unlink(WSP_ALERT_PATH);
    log_transaction("upgrade", "Базовая система и ядро успешно обновлены");

    print_step(
        total_steps,
        total_steps,
        t(
            "Обновление конфигурации загрузчика Limine",
            "Updating Limine bootloader configuration",
        ),
        true,
    );
    println!("wsp: системное обновление успешно завершено.");
    println!("  Для перехода на обновленное ядро перезагрузите систему (reboot).");

    0
}

fn apply_kernel_rotation_with(new_ver: &Version, old_ver: &Version, root_dir: &str) {
    let current_entry = KernelEntry {
        version: *new_ver,
        kernel_path: "boot():/boot/wunix-kernel".into(),
        cmdline: "root=/dev/vda1 lang=ru mode=disk".into(),
    };

    let fallback_entry = KernelEntry {
        version: *old_ver,
        kernel_path: "boot():/boot/wunix-kernel.old".into(),
        cmdline: "root=/dev/vda1 lang=ru mode=disk".into(),
    };

    let new_conf = generate_limine_conf(&current_entry, Some(&fallback_entry), 3);

    let conf_path = if root_dir.is_empty() {
        "/boot/limine.conf".to_string()
    } else {
        format!("{}/boot/limine.conf", root_dir.trim_end_matches('/'))
    };

    let mut null_p = [0u8; 256];
    let b = conf_path.as_bytes();
    let l = b.len().min(255);
    null_p[..l].copy_from_slice(&b[..l]);
    null_p[l] = 0;

    let fd = open(&null_p, (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let _ = write(fd as u64, new_conf.as_bytes());
        close(fd as u64);
    }
}
