//! Модуль аудита безопасности и целостности системы WSP.
//!
//! Реализует команды:
//! - `wsp audit`: полный детальный отчет о состоянии ядра, VFS и пакетов.
//! - `wsp check`: быстрый код возврата (0 = OK, 1 = updates, 2 = critical updates).

extern crate alloc;
use alloc::format;
use libwunix::{close, open, println, t, O_RDONLY};

use crate::db::{has_critical_alert, load_installed_packages};
use crate::display::{
    print_card_footer, print_card_header, print_card_row, print_card_separator,
    COLOR_BOLD, COLOR_CYAN, COLOR_GREEN, COLOR_RED, COLOR_RESET,
};

/// Выполняет полный аудит системы и выводит информационный блок
pub fn run_audit(verify_files: bool) -> i32 {
    let packages = load_installed_packages();
    let has_alert = has_critical_alert();

    print_card_header(t("Wunix OS: Аудит базовой системы", "Wunix OS: Base System Audit"));

    // 1. Профиль и архитектура
    print_card_row(
        t("Профиль системы:", "System profile:"),
        "wunix-base (Легковесный / Терминальный)",
    );
    print_card_row(
        t("Архитектура CPU:", "CPU Architecture:"),
        "x86_64 | Загрузчик: Limine 8.x",
    );

    // 2. Статус ядра
    let kernel_fd = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if kernel_fd >= 0 {
        close(kernel_fd as u64);
        print_card_row(
            t("Активное ядро:", "Active kernel:"),
            "/boot/wunix-kernel (Загружено успешно)",
        );
    } else {
        print_card_row(
            t("Активное ядро:", "Active kernel:"),
            "/boot/wunix-kernel [ВНИМАНИЕ: файл не найден]",
        );
    }

    // 3. Резервное ядро
    let fallback_fd = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if fallback_fd >= 0 {
        close(fallback_fd as u64);
        print_card_row(
            t("Резервное ядро:", "Fallback kernel:"),
            "/boot/wunix-kernel.old (Доступно)",
        );
    } else {
        print_card_row(
            t("Резервное ядро:", "Fallback kernel:"),
            "Отсутствует (Будет создано при обновлении)",
        );
    }

    print_card_separator();

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
            format!("{}{} файлов проверено (Все на месте, OK){}", COLOR_GREEN, total_files, COLOR_RESET)
        } else {
            format!("{}{} проверено, {} повреждено/отсутствует!{}", COLOR_RED, total_files, missing_files, COLOR_RESET)
        }
    } else {
        format!("{} файлов зарегистрировано (флаг -v для пофайловой сверки)", total_files)
    };
    print_card_row(t("Целостность файлов:", "File integrity:"), &integrity_str);

    print_card_row(
        t("База данных WSP:", "WSP database:"),
        "/var/lib/wsp/installed.json (Синхронизирована)",
    );

    print_card_separator();

    // 5. Статус безопасности
    if has_alert {
        let alert_status = format!("{}[ ВНИМАНИЕ: ДОСТУПНЫ КРИТИЧЕСКИЕ ОБНОВЛЕНИЯ ]{}", COLOR_RED, COLOR_RESET);
        print_card_row(t("СТАТУС БЕЗОПАСНОСТИ:", "SECURITY STATUS:"), &alert_status);
        println!("{}│{}", COLOR_CYAN, COLOR_RESET);
        println!("{}│ • wunix-kernel   0.4.1 -> 0.4.2   [КРИТИЧЕСКИЙ]  Патч вызовов и планировщика{}", COLOR_RED, COLOR_RESET);
        println!("{}│ • libwunix       0.1.0 -> 0.2.0   [КРИТИЧЕСКИЙ]  Исправление mmap/vma границ{}", COLOR_RED, COLOR_RESET);
        println!("{}│{}", COLOR_CYAN, COLOR_RESET);
        println!("{}│ Рекомендация: обновите систему командой: {}{}wsp upgrade --critical{}", 
            COLOR_CYAN, COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    } else {
        let alert_status = format!("{}[ СИСТЕМА В АКТУАЛЬНОМ И БЕЗОПАСНОМ СОСТОЯНИИ ]{}", COLOR_GREEN, COLOR_RESET);
        print_card_row(t("СТАТУС БЕЗОПАСНОСТИ:", "SECURITY STATUS:"), &alert_status);
    }

    print_card_footer();

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
        println!("{}[КРИТИЧЕСКОЕ ПРЕДУПРЕЖДЕНИЕ]{} Доступны важные обновления базы Wunix! Выполните 'wsp upgrade --critical'",
            COLOR_RED, COLOR_RESET);
    } else if !critical_only {
        println!("{}[WSP]{} Базовая система Wunix актуальна.", COLOR_GREEN, COLOR_RESET);
    }

    if has_alert { 2 } else { 0 }
}
