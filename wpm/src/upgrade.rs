//! Единый модуль обновления системных (включая ядро) и прикладных пакетов WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{
    close, geteuid, getuid, mkdir, mount, open, println, t, umount2, write, O_CREAT, O_TRUNC,
    O_WRONLY,
};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::sources::get_wsp_system_repo;
use libpackwunix::version::Version;

use crate::db::{
    clear_critical_alerts, copy_file, ensure_dir_recursive, get_path, load_installed_packages_from,
    load_unified_catalog_from, log_transaction, save_installed_packages_to, PackageEntry,
};
use crate::display::{
    confirm_prompt, format_size, print_access_denied, print_disk_space_change, print_error,
    print_step, COLOR_GREEN, COLOR_RED, COLOR_RESET,
};
use crate::guard::ensure_system_safe_for_user_apps;
use crate::install::locate_local_package_file;
use crate::unpack::{read_file_to_vec, unpack_package_to};

struct UpgradableItem {
    name: String,
    channel: String,
    pkg_type: String,
    tier: String,
    old_version: Version,
    new_version: Version,
    size: u64,
    filename: String,
    sha256: String,
}

fn resolve_channel_url(channel: &str, filename: &str) -> String {
    if channel == "system" {
        return format!("{}/{}", get_wsp_system_repo().trim_end_matches('/'), filename);
    }
    let sources = crate::repo_cmd::load_sources();
    if let Some(sec) = sources.iter().find(|s| s.name == channel) {
        if let Some(srv) = sec.servers.first() {
            return format!("{}/{}", srv.trim_end_matches('/'), filename);
        }
    }
    let base = sources
        .first()
        .and_then(|s| s.servers.first())
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("http://wunix.rusos.site/repo/apps/x86_64");
    format!("{}/{}", base, filename)
}

pub fn run_upgrade(
    root_prefix: Option<&str>,
    critical_only: bool,
    assume_yes: bool,
    ignore_unsafe: bool,
    dry_run: bool,
    verbose: bool,
) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("upgrade", uid);
        return 1;
    }

    let root_str = root_prefix.unwrap_or("/");
    println!("wpm: проверка доступных обновлений пакетов...");

    let installed = load_installed_packages_from(root_str);
    let catalog = load_unified_catalog_from(root_str);

    let mut upgradable: Vec<UpgradableItem> = Vec::new();
    for inst in &installed {
        let inst_ver = Version::parse(&inst.version).unwrap_or(Version::new(0, 1, 0));
        if let Some(entry) = catalog.iter().find(|c| c.pkg.name == inst.name) {
            let rp = &entry.pkg;
            if rp.version.is_newer_than(&inst_ver) {
                let is_critical = rp.tier == "critical" || rp.name == "wunix-kernel";
                if critical_only && !is_critical {
                    continue;
                }
                upgradable.push(UpgradableItem {
                    name: inst.name.clone(),
                    channel: entry.channel.clone(),
                    pkg_type: rp.package_type.clone(),
                    tier: rp.tier.clone(),
                    old_version: inst_ver,
                    new_version: rp.version,
                    size: rp.size,
                    filename: rp.filename.clone(),
                    sha256: rp.sha256.clone(),
                });
            }
        }
    }

    if upgradable.is_empty() {
        if critical_only {
            println!("wpm: все критические компоненты ядра находятся в актуальном состоянии.");
        } else {
            println!("wpm: все установленные пакеты имеют актуальные версии.");
        }
        return 0;
    }

    let has_only_user = upgradable.iter().all(|u| u.pkg_type == "user");
    if has_only_user && root_prefix.is_none() && ensure_system_safe_for_user_apps(ignore_unsafe).is_err() {
        return 1;
    }

    println!("{}:", t("Пакеты, подготовленные к обновлению", "Packages to be upgraded"));
    let mut total_download: u64 = 0;
    let mut total_installed: u64 = 0;
    for item in &upgradable {
        total_download += item.size;
        total_installed += item.size.saturating_mul(2);
        let badge = if item.tier == "critical" {
            format!("{}[критический]{}", COLOR_RED, COLOR_RESET)
        } else {
            format!("{}[{}]{}", COLOR_GREEN, item.channel, COLOR_RESET)
        };
        println!(
            "  - {:<16} v{} -> v{} {} ({})",
            item.name,
            item.old_version,
            item.new_version,
            badge,
            format_size(item.size)
        );
    }

    print_disk_space_change(total_download, total_installed, true);

    if dry_run {
        println!("  Режим симуляции (--dry-run). Файлы не изменялись.");
        return 0;
    }

    if !assume_yes {
        let prompt_text = t("Применить обновление пакетов? [Y/n] ", "Apply package upgrades? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: обновление отменено пользователем.", "wpm: upgrade aborted by user."));
            return 0;
        }
    }

    let cache_dir = get_path(root_str, "/var/cache/wpm");
    ensure_dir_recursive(&cache_dir);

    let mut new_installed = installed.clone();
    let total_steps = upgradable.len();
    let mut kernel_updated = false;

    for (idx, item) in upgradable.iter().enumerate() {
        let step_label = format!("Обновление {}", item.name);

        let pkg_data = if let Some(local_path) = locate_local_package_file(&item.name, Some(&item.filename), root_prefix) {
            match read_file_to_vec(&local_path) {
                Ok(d) => d,
                Err(e) => {
                    print_step(idx + 1, total_steps, &step_label, false);
                    print_error(e);
                    return 1;
                }
            }
        } else {
            let dest_path = format!("{}/{}", cache_dir, item.filename);
            let download_url = resolve_channel_url(&item.channel, &item.filename);
            println!("wpm: загрузка '{}' ({})...", item.name, format_size(item.size));
            let sha_opt = if item.sha256.is_empty() { None } else { Some(item.sha256.as_str()) };
            if let Err(e) = crate::http::fetch_to_file(&download_url, &dest_path, sha_opt, &item.filename) {
                print_step(idx + 1, total_steps, &step_label, false);
                print_error(&format!("не удалось загрузить {}: {:?}", item.filename, e));
                return 1;
            }
            match read_file_to_vec(&dest_path) {
                Ok(d) => d,
                Err(e) => {
                    print_step(idx + 1, total_steps, &step_label, false);
                    print_error(e);
                    return 1;
                }
            }
        };

        if item.name == "wunix-kernel" {
            let cur_k = get_path(root_str, "/boot/wunix-kernel");
            let old_k = get_path(root_str, "/boot/wunix-kernel.old");
            let _ = copy_file(&cur_k, &old_k);
        }

        let (manifest, files) = match unpack_package_to(&pkg_data, root_prefix, verbose) {
            Ok(res) => res,
            Err(e) => {
                print_step(idx + 1, total_steps, &step_label, false);
                print_error(e);
                return 1;
            }
        };

        if item.name == "wunix-kernel" {
            apply_kernel_rotation_with(&item.new_version, &item.old_version, root_str);
            kernel_updated = true;
        }

        let deps: Vec<String> = manifest.required_deps.iter().map(|d| d.name.clone()).collect();
        new_installed.retain(|a| a.name != manifest.name);
        new_installed.push(PackageEntry {
            name: manifest.name.clone(),
            version: manifest.version.to_string(),
            pkg_type: manifest.package_type.as_str().to_string(),
            tier: manifest.tier.as_str().to_string(),
            is_meta: manifest.is_meta,
            description: manifest.description.clone(),
            dependencies: deps,
            files,
        });
        save_installed_packages_to(root_str, &new_installed);
        print_step(idx + 1, total_steps, &step_label, true);
    }

    if upgradable.iter().any(|u| u.pkg_type == "system") {
        clear_critical_alerts();
    }

    log_transaction("upgrade", &format!("Обновлено пакетов: {}", upgradable.len()));
    println!("wpm: обновление пакетов успешно завершено.");
    if kernel_updated {
        println!("  Для перехода на обновленное ядро перезагрузите систему (reboot).");
    }
    0
}

fn apply_kernel_rotation_with(new_ver: &Version, old_ver: &Version, root_dir: &str) {
    let mut cmdline = "root=/dev/vda1 lang=ru mode=disk".to_string();
    if let Ok(c) = read_file_to_vec("/proc/cmdline") {
        let s = core::str::from_utf8(&c).unwrap_or("").trim();
        if !s.is_empty() {
            cmdline = s.to_string();
        }
    }

    let current_entry = KernelEntry {
        version: *new_ver,
        kernel_path: "boot():/boot/wunix-kernel".into(),
        cmdline: cmdline.clone(),
    };
    let fallback_entry = KernelEntry {
        version: *old_ver,
        kernel_path: "boot():/boot/wunix-kernel.old".into(),
        cmdline,
    };

    let new_conf = generate_limine_conf(&current_entry, Some(&fallback_entry), 3);
    let conf_path = get_path(root_dir, "/boot/limine.conf");

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

    sync_kernel_to_boot_partition(root_dir);
}

fn sync_kernel_to_boot_partition(root_dir: &str) {
    let candidate_devs: &[&[u8]] = &[b"/dev/vda1\0", b"/dev/sda1\0"];
    let mnt_boot = b"/mnt_boot\0";
    let _ = mkdir(mnt_boot, 0o755);

    for dev in candidate_devs {
        if mount(*dev, mnt_boot, b"fat32\0", 0, b"\0") >= 0 {
            let kernel_src = get_path(root_dir, "/boot/wunix-kernel");
            let _ = copy_file(&kernel_src, "/mnt_boot/boot/wunix-kernel");

            let old_src = get_path(root_dir, "/boot/wunix-kernel.old");
            let _ = copy_file(&old_src, "/mnt_boot/boot/wunix-kernel.old");

            let conf_src = get_path(root_dir, "/boot/limine.conf");
            let _ = copy_file(&conf_src, "/mnt_boot/boot/limine.conf");

            let _ = umount2(mnt_boot, 0);
            println!(
                "  wpm: ядро успешно синхронизировано с загрузочным разделом {}",
                core::str::from_utf8(dev).unwrap_or("").trim_end_matches('\0')
            );
            break;
        }
    }
}
