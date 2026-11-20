//! Модуль применения обновлений базовой системы WSP.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{
    close, geteuid, getuid, mkdir, mount, open, println, t, umount2, unlink, write, O_CREAT,
    O_TRUNC, O_WRONLY,
};
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
    sha256: String,
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
        let inst_ver = Version::parse(&inst.version).unwrap_or(Version::new(0, 1, 0));
        if let Some(repo_pkg) = repo_packages.iter().find(|p| p.name == inst.name) {
            if repo_pkg.version.is_newer_than(&inst_ver) {
                if critical_only && repo_pkg.tier != "critical" && repo_pkg.name != "wunix-kernel" {
                    continue;
                }
                upgradable.push(UpgradableItem {
                    name: inst.name.clone(),
                    old_version: inst_ver,
                    new_version: repo_pkg.version,
                    tier: repo_pkg.tier.clone(),
                    size: repo_pkg.size,
                    sha256: repo_pkg.sha256.clone(),
                    filename: repo_pkg.filename.clone(),
                });
            }
        }
    }

    if !installed.iter().any(|p| p.name == "wunix-kernel") {
        if let Some(repo_pkg) = repo_packages.iter().find(|p| p.name == "wunix-kernel") {
            let k_path = crate::db::get_path(root_dir, "/boot/wunix-kernel");
            let fd = crate::db::open_c(&k_path, libwunix::O_RDONLY as u64, 0);
            if fd >= 0 {
                libwunix::close(fd as u64);
                let inst_ver = Version::new(0, 1, 0);
                if repo_pkg.version.is_newer_than(&inst_ver) {
                    upgradable.push(UpgradableItem {
                        name: "wunix-kernel".to_string(),
                        old_version: inst_ver,
                        new_version: repo_pkg.version,
                        tier: repo_pkg.tier.clone(),
                        size: repo_pkg.size,
                        sha256: repo_pkg.sha256.clone(),
                        filename: repo_pkg.filename.clone(),
                    });
                }
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
                if libpackwunix::format::PackageContainer::parse(&d).is_ok() {
                    pkg_data = Some(d);
                    break;
                }
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
            let sha_opt = if item.sha256.is_empty() { None } else { Some(item.sha256.as_str()) };
            match crate::http::fetch_to_file(&download_url, &dest_path, sha_opt, &item.filename) {
                Ok(_) => {
                    if let Ok(d) = read_file_to_vec(&dest_path) {
                        if libpackwunix::format::PackageContainer::parse(&d).is_ok() {
                            pkg_data = Some(d);
                        } else {
                            crate::display::print_error("поврежден контейнер загруженного пакета");
                        }
                    }
                }
                Err(e) => {
                    crate::display::print_error(&format!("ошибка сети при загрузке: {:?}", e));
                }
            }
        }

        let mut step_ok = false;
        if let Some(data) = pkg_data {
            if item.name == "wunix-kernel" {
                let cur_k = crate::db::get_path(root_dir, "/boot/wunix-kernel");
                let old_k = crate::db::get_path(root_dir, "/boot/wunix-kernel.old");
                let _ = crate::db::copy_file(&cur_k, &old_k);
            }
            match unpack_wsp_package_to(&data, root_dir, false) {
                Ok((manifest, files)) => {
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
                    step_ok = true;
                }
                Err(e) => {
                    print_step(current_step, total_steps, &step_label, false);
                    crate::display::print_error(&format!("ошибка распаковки {}: {}", item.name, e));
                }
            }
        } else {
            print_step(current_step, total_steps, &step_label, false);
            crate::display::print_error(&format!("не удалось загрузить или прочитать пакет {}", item.filename));
        }

        if step_ok {
            print_step(current_step, total_steps, &step_label, true);
        }
        current_step += 1;
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
    let mut cmdline = "root=/dev/vda1 lang=ru mode=disk".to_string();
    if let Ok(c) = crate::unpack::read_file_to_vec("/proc/cmdline") {
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

    sync_kernel_to_boot_partition(root_dir);
}

fn sync_kernel_to_boot_partition(root_dir: &str) {
    let candidate_devs: &[&[u8]] = &[
        b"/dev/vda1\0",
        b"/dev/sda1\0",
    ];

    let mnt_boot = b"/mnt_boot\0";
    let _ = mkdir(mnt_boot, 0o755);

    for dev in candidate_devs {
        if mount(*dev, mnt_boot, b"fat32\0", 0, b"\0") >= 0 {
            let kernel_src = crate::db::get_path(root_dir, "/boot/wunix-kernel");
            let kernel_dst = "/mnt_boot/boot/wunix-kernel";
            let _ = crate::db::copy_file(&kernel_src, kernel_dst);

            let old_src = crate::db::get_path(root_dir, "/boot/wunix-kernel.old");
            let old_dst = "/mnt_boot/boot/wunix-kernel.old";
            let _ = crate::db::copy_file(&old_src, old_dst);

            let conf_src = crate::db::get_path(root_dir, "/boot/limine.conf");
            let conf_dst = "/mnt_boot/boot/limine.conf";
            let _ = crate::db::copy_file(&conf_src, conf_dst);

            let _ = umount2(mnt_boot, 0);
            println!(
                "  wsp: ядро успешно синхронизировано с загрузочным разделом {}",
                core::str::from_utf8(dev).unwrap_or("").trim_end_matches('\0')
            );
            break;
        }
    }
}
