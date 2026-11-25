//! Модуль аудита безопасности и проверки обновлений единого WPM.

extern crate alloc;
use alloc::format;
use alloc::vec::Vec;
use libwunix::{close, open, println, t, O_RDONLY};
use libpackwunix::version::Version;

use crate::db::{has_critical_alert, load_installed_packages, load_unified_catalog_from};
use crate::display::{COLOR_BOLD, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};

pub fn run_audit(verify_files: bool) -> i32 {
    let packages = load_installed_packages();
    let has_alert = has_critical_alert();

    println!("{}:", t("Аудит целостности и безопасности системы Wunix", "Wunix system integrity and security audit"));
    println!("  {:<22} {}", t("Профиль платформы:", "System profile:"), "Wunix OS (x86_64)");

    let kernel_fd = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if kernel_fd >= 0 {
        close(kernel_fd as u64);
        println!("  {:<22} {}/boot/wunix-kernel [активно]{}", t("Активное ядро:", "Active kernel:"), COLOR_GREEN, COLOR_RESET);
    } else {
        println!("  {:<22} {}/boot/wunix-kernel [не найдено!]{}", t("Активное ядро:", "Active kernel:"), COLOR_RED, COLOR_RESET);
    }

    let fallback_fd = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if fallback_fd >= 0 {
        close(fallback_fd as u64);
        println!("  {:<22} {}/boot/wunix-kernel.old [готово к откату]{}", t("Резервное ядро:", "Fallback kernel:"), COLOR_GREEN, COLOR_RESET);
    } else {
        println!("  {:<22} {}", t("Резервное ядро:", "Fallback kernel:"), t("отсутствует (создается при ротации)", "none (created on rotation)"));
    }

    let mut total_files = 0;
    let mut missing_files = 0;

    for pkg in &packages {
        total_files += pkg.files.len();
        if verify_files {
            for file_path in &pkg.files {
                let mut path_bytes = [0u8; 256];
                let b = file_path.as_bytes();
                let len = b.len().min(255);
                path_bytes[..len].copy_from_slice(&b[..len]);
                path_bytes[len] = 0;

                let fd = open(&path_bytes, O_RDONLY as u64, 0);
                if fd >= 0 {
                    close(fd as u64);
                } else {
                    missing_files += 1;
                }
            }
        }
    }

    let integrity_str = if verify_files {
        if missing_files == 0 {
            format!("{}{} файлов проверено, повреждений не обнаружено{}", COLOR_GREEN, total_files, COLOR_RESET)
        } else {
            format!("{}{} проверено, {} отсутствует!{}", COLOR_RED, total_files, missing_files, COLOR_RESET)
        }
    } else {
        format!("{} файлов учтено (используйте -v для сверки)", total_files)
    };
    println!("  {:<22} {}", t("Целостность файлов:", "File integrity:"), integrity_str);
    println!("  {:<22} {}", t("Конфигурация WPM:", "WPM config:"), "/etc/wpm/wpm.conf");
    println!("  {:<22} {}", t("База реестра:", "Registry db:"), "/var/lib/wpm/installed.json");

    if has_alert {
        println!("  {:<22} {}{}{}", t("Статус безопасности:", "Security status:"), COLOR_RED, t("[Требуются критические обновления]", "[Critical updates pending]"), COLOR_RESET);
        println!("  Рекомендация: sudo wpm upgrade --critical");
        2
    } else {
        println!("  {:<22} {}{}{}", t("Статус безопасности:", "Security status:"), COLOR_GREEN, t("[Система актуальна и защищена]", "[System is secure and up to date]"), COLOR_RESET);
        0
    }
}

pub fn run_check(root_prefix: Option<&str>, critical_only: bool, json_output: bool) -> i32 {
    let root_str = root_prefix.unwrap_or("/");
    let installed = load_installed_packages();
    let catalog = load_unified_catalog_from(root_str);

    let mut updates_available = 0;
    let mut critical_count = 0;
    let mut details: Vec<alloc::string::String> = Vec::new();

    for inst in &installed {
        let clean_ver = inst.version.trim_start_matches(|c| c == 'v' || c == 'V');
        let inst_ver = Version::parse(clean_ver).unwrap_or(Version::new(0, 1, 0));
        if let Some(entry) = catalog.iter().find(|c| c.pkg.name == inst.name) {
            if entry.pkg.version.is_newer_than(&inst_ver) {
                let is_critical = entry.pkg.tier == "critical" || entry.pkg.name == "wunix-kernel";
                if critical_only && !is_critical {
                    continue;
                }
                updates_available += 1;
                if is_critical {
                    critical_count += 1;
                }
                details.push(format!(
                    "- {:<16} v{} -> v{} [{}]",
                    inst.name, inst_ver, entry.pkg.version, entry.channel
                ));
            }
        }
    }

    let has_alert = has_critical_alert() || critical_count > 0;

    if json_output {
        if updates_available > 0 || has_alert {
            println!("{{\"updates_available\": true, \"critical\": {}, \"count\": {}}}", has_alert, updates_available);
        } else {
            println!("{{\"updates_available\": false, \"critical\": false, \"count\": 0}}");
        }
    } else if updates_available > 0 {
        for d in &details {
            println!("{}", d);
        }
    } else if has_alert {
        println!(
            "{}{}Внимание:{} {}",
            COLOR_BOLD,
            COLOR_YELLOW,
            COLOR_RESET,
            t("доступны критические системные обновления ядра!", "critical kernel updates are available!")
        );
        println!("  Выполните: sudo wpm upgrade --critical");
    } else if !critical_only {
        println!("{}", t("wpm: все установленные пакеты обновлены до актуальных версий.", "wpm: all installed packages are up to date."));
    }

    if has_alert { 2 } else { 0 }
}
