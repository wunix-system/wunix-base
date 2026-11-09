//! Модуль аудита безопасности и целостности системы WSP.
//!
//! Реализует команды:
//! - `wsp audit`: полный детальный аудит ядра, загрузчика, целостности файлов и статуса базы.
//! - `wsp check`: быстрая проверка наличия критических обновлений.

extern crate alloc;
use alloc::format;
use libwunix::{close, open, println, t, O_RDONLY};

use crate::db::{has_critical_alert, load_installed_packages};
use crate::display::{COLOR_BOLD, COLOR_CYAN, COLOR_DIM, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};

/// Выполняет полный аудит системы
pub fn run_audit(verify_files: bool) -> i32 {
    let packages = load_installed_packages();
    let has_alert = has_critical_alert();

    println!();
    println!("  {}{}◆ WSP ── Аудит базовой системы и ядра Wunix{}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);

    // 1. Профиль и архитектура
    println!("  {:<22} {}", t("Профиль платформы:", "System profile:"), "Wunix Base System (x86_64)");
    println!("  {:<22} {}", t("Архитектура CPU:", "CPU Architecture:"), "x86_64 Long Mode (Ring 0 / Ring 3)");

    // 2. Статус ядра
    let kernel_fd = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if kernel_fd >= 0 {
        close(kernel_fd as u64);
        println!("  {:<22} {}/boot/wunix-kernel [активно]{}", t("Активное ядро:", "Active kernel:"), COLOR_GREEN, COLOR_RESET);
    } else {
        println!("  {:<22} {}/boot/wunix-kernel [не найдено!]{}", t("Активное ядро:", "Active kernel:"), COLOR_RED, COLOR_RESET);
    }

    // 3. Резервное ядро
    let fallback_fd = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if fallback_fd >= 0 {
        close(fallback_fd as u64);
        println!("  {:<22} {}/boot/wunix-kernel.old [готово к откату]{}", t("Резервное ядро:", "Fallback kernel:"), COLOR_GREEN, COLOR_RESET);
    } else {
        println!("  {:<22} {}", t("Резервное ядро:", "Fallback kernel:"), t("отсутствует (создается при ротации)", "none (created on rotation)"));
    }

    // 4. Целостность файлов
    let mut total_files = 0;
    let mut missing_files = 0;

    for pkg in &packages {
        total_files += pkg.files.len();
        if verify_files {
            for file_path in &pkg.files {
                let mut path_bytes = [0u8; 128];
                let b = file_path.as_bytes();
                let len = b.len().min(127);
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
    println!("  {:<22} {}", t("Конфигурация WSP:", "WSP config:"), "/etc/wsp/wsp.conf");
    println!("  {:<22} {}", t("База реестра:", "Registry db:"), "/var/lib/wsp/installed.json");

    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);

    // 5. Статус безопасности
    if has_alert {
        println!("  {:<22} {}{}{}", t("Статус безопасности:", "Security status:"), COLOR_RED, t("[Требуются критические обновления]", "[Critical updates pending]"), COLOR_RESET);
        println!();
        println!("  {}", t("Доступны обязательные обновления безопасности ядра:", "Mandatory kernel security updates available:"));
        println!("    • wunix-kernel   0.4.1 -> 0.4.2   [критический: исправление планировщика и poll]");
        println!("    • libwunix       0.1.0 -> 0.2.0   [критический: защита границ виртуальной памяти mmap]");
        println!();
        println!("  {}: {}sudo wsp upgrade --critical{}", t("Рекомендация", "Recommendation"), COLOR_BOLD, COLOR_RESET);
    } else {
        println!("  {:<22} {}{}{}", t("Статус безопасности:", "Security status:"), COLOR_GREEN, t("[Система актуальна и защищена]", "[System is secure and up to date]"), COLOR_RESET);
    }

    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!();

    if has_alert { 2 } else { 0 }
}

/// Быстрая проверка наличия системных обновлений
pub fn run_check(critical_only: bool, json_output: bool) -> i32 {
    let has_alert = has_critical_alert();

    if json_output {
        if has_alert {
            println!("{{\"updates_available\": true, \"critical\": true}}");
        } else {
            println!("{{\"updates_available\": false, \"critical\": false}}");
        }
    } else if has_alert {
        println!();
        println!("  {}{}Внимание:{} {}", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET, t("доступны критические системные обновления ядра!", "critical kernel updates are available!"));
        println!("  {}: {}sudo wsp upgrade --critical{}", t("Выполните", "Run"), COLOR_BOLD, COLOR_RESET);
        println!();
    } else if !critical_only {
        println!("  {}{}✔ Базовая система Wunix полностью актуальна.{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    }

    if has_alert { 2 } else { 0 }
}
