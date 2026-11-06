//! Модуль аудита безопасности и целостности системы WSP.
//!
//! Реализует команды:
//! - `wsp audit`: полный детальный отчет о состоянии ядра, VFS и пакетов.
//! - `wsp check`: быстрый код возврата (0 = OK, 1 = updates, 2 = critical updates).

extern crate alloc;
use alloc::format;
use libwunix::{close, open, println, t, O_RDONLY};

use crate::db::{has_critical_alert, load_installed_packages};
use crate::display::{print_field, print_warning, COLOR_BOLD, COLOR_GREEN, COLOR_RED, COLOR_RESET};

/// Выполняет полный аудит системы и выводит информационный блок
pub fn run_audit(verify_files: bool) -> i32 {
    let packages = load_installed_packages();
    let has_alert = has_critical_alert();

    println!("{}", t("Аудит базовой системы Wunix:", "Wunix Base System Audit:"));

    // 1. Профиль и архитектура
    print_field(
        t("Профиль системы:", "System profile:"),
        t("wunix-base (терминальный)", "wunix-base (terminal)"),
    );
    print_field(
        t("Архитектура CPU:", "CPU Architecture:"),
        "x86_64 (Limine 8.x)",
    );

    // 2. Статус ядра
    let kernel_fd = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if kernel_fd >= 0 {
        close(kernel_fd as u64);
        print_field(
            t("Активное ядро:", "Active kernel:"),
            "/boot/wunix-kernel (загружено)",
        );
    } else {
        print_field(
            t("Активное ядро:", "Active kernel:"),
            &format!("{}/boot/wunix-kernel (не найдено!){}", COLOR_RED, COLOR_RESET),
        );
    }

    // 3. Резервное ядро
    let fallback_fd = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if fallback_fd >= 0 {
        close(fallback_fd as u64);
        print_field(
            t("Резервное ядро:", "Fallback kernel:"),
            "/boot/wunix-kernel.old (доступно для отката)",
        );
    } else {
        print_field(
            t("Резервное ядро:", "Fallback kernel:"),
            t("отсутствует (создается при обновлении)", "none (created on upgrade)"),
        );
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
            format!("{}{} проверено, все файлы на месте{}", COLOR_GREEN, total_files, COLOR_RESET)
        } else {
            format!("{}{} проверено, {} повреждено или отсутствует!{}", COLOR_RED, total_files, missing_files, COLOR_RESET)
        }
    } else {
        format!("{} файлов зарегистрировано (используйте -v для сверки)", total_files)
    };
    print_field(t("Целостность файлов:", "File integrity:"), &integrity_str);

    print_field(
        t("База данных WSP:", "WSP database:"),
        "/var/lib/wsp/installed.json",
    );

    println!();
    println!("{}", t("Безопасность и обновления:", "Security & Updates:"));

    // 5. Статус безопасности
    if has_alert {
        print_field(
            t("Статус системы:", "System status:"),
            &format!("{}[требуются критические обновления]{}", COLOR_RED, COLOR_RESET),
        );
        println!();
        println!("  {}", t("Доступны обязательные обновления ядра и базовой системы:", "Mandatory kernel and base updates available:"));
        println!("    • wunix-kernel   0.4.1 -> 0.4.2   (критическое: исправление планировщика и poll)");
        println!("    • libwunix       0.1.0 -> 0.2.0   (критическое: исправление границ mmap/vma)");
        println!();
        println!("  {}: {}wsp upgrade --critical{}", t("Рекомендация", "Recommendation"), COLOR_BOLD, COLOR_RESET);
    } else {
        print_field(
            t("Статус системы:", "System status:"),
            &format!("{}[система актуальна и защищена]{}", COLOR_GREEN, COLOR_RESET),
        );
    }

    if has_alert { 2 } else { 0 }
}

/// Быстрая проверка наличия системных обновлений без вывода лишней информации
pub fn run_check(critical_only: bool, json_output: bool) -> i32 {
    let has_alert = has_critical_alert();

    if json_output {
        if has_alert {
            println!("{{\"updates_available\": true, \"critical\": true}}");
        } else {
            println!("{{\"updates_available\": false, \"critical\": false}}");
        }
    } else if has_alert {
        print_warning(t(
            "доступны критические системные обновления! Выполните 'wsp upgrade --critical'",
            "critical system updates available! Run 'wsp upgrade --critical'",
        ));
    } else if !critical_only {
        println!("{}", t("wsp: базовая система Wunix актуальна.", "wsp: Wunix base system is up to date."));
    }

    if has_alert { 2 } else { 0 }
}
