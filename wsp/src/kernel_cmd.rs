//! Модуль управления ядрами WSP (`wsp kernel`).
//!
//! Реализует:
//! - `wsp kernel list`: инвентаризация ядер в `/boot`.
//! - `wsp kernel rollback`: экстренный откат загрузчика на резервное ядро.
//! - `wsp kernel clean`: очистка устаревших ядер.

extern crate alloc;
use libwunix::{close, open, println, unlink, write, O_CREAT, O_RDONLY, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::log_transaction;
use crate::display::{COLOR_BOLD, COLOR_CYAN, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};

/// Обработка подкоманд управления ядром
pub fn run_kernel_cmd(subcmd: &str) -> i32 {
    match subcmd {
        "list" | "ls" => list_kernels(),
        "rollback" | "revert" => rollback_kernel(),
        "clean" | "gc" => clean_kernels(),
        _ => {
            println!("{}[!] Неизвестная подкоманда управления ядром: '{}'{}", COLOR_RED, subcmd, COLOR_RESET);
            println!("Использование: wsp kernel <list | rollback | clean>");
            1
        }
    }
}

/// Выводит список ядер, доступных в каталоге /boot
fn list_kernels() -> i32 {
    println!("{}{}Ядра Wunix в каталоге /boot:{}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);

    let k_main = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if k_main >= 0 {
        close(k_main as u64);
        println!("  • {}/boot/wunix-kernel{}        {}[АКТИВНОЕ / DEFAULT]{}", COLOR_BOLD, COLOR_RESET, COLOR_GREEN, COLOR_RESET);
    } else {
        println!("  • /boot/wunix-kernel        {}[ОТСУТСТВУЕТ]{}", COLOR_RED, COLOR_RESET);
    }

    let k_old = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if k_old >= 0 {
        close(k_old as u64);
        println!("  • {}/boot/wunix-kernel.old{}    {}[РЕЗЕРВНОЕ / FALLBACK]{}", COLOR_BOLD, COLOR_RESET, COLOR_YELLOW, COLOR_RESET);
    } else {
        println!("  • /boot/wunix-kernel.old    {}[НЕ СОЗДАНО]{}", COLOR_RESET, COLOR_RESET);
    }

    0
}

/// Выполняет откат на резервное ядро в limine.conf
fn rollback_kernel() -> i32 {
    println!("{}[WSP]{} Активация резервного ядра в конфигурации Limine...", COLOR_CYAN, COLOR_RESET);

    let fallback_entry = KernelEntry {
        version: Version::new(0, 4, 1),
        kernel_path: "boot():/boot/wunix-kernel.old".into(),
        cmdline: "root=/dev/vda1 lang=ru mode=disk".into(),
    };

    let conf = generate_limine_conf(&fallback_entry, None, 3);
    let fd = open(b"/boot/limine.conf\0", (O_WRONLY | O_CREAT) as u64, 0o644);
    if fd >= 0 {
        let b = conf.as_bytes();
        let _ = write(fd as u64, b);
        close(fd as u64);
        log_transaction("kernel-rollback", "Загрузчик переключен на резервное ядро");
        println!("{}{}✓ Успешно! Резервное ядро назначено загрузочным по умолчанию.{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
        println!("Перезагрузите машину для запуска: reboot");
        0
    } else {
        println!("{}[!] Ошибка записи в /boot/limine.conf{}", COLOR_RED, COLOR_RESET);
        1
    }
}

/// Удаляет устаревшие резервные ядра для экономии дискового пространства
fn clean_kernels() -> i32 {
    println!("{}[WSP]{} Очистка старых резервных ядер...", COLOR_CYAN, COLOR_RESET);
    let _ = unlink(b"/boot/wunix-kernel.old\0");
    log_transaction("kernel-clean", "Очистка старых ядер завершена");
    println!("{}{}✓ Очистка завершена.{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    0
}
