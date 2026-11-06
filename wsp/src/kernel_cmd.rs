//! Модуль управления ядрами WSP (`wsp kernel`).
//!
//! Реализует:
//! - `wsp kernel list`: инвентаризация ядер в `/boot`.
//! - `wsp kernel rollback`: экстренный откат загрузчика на резервное ядро.
//! - `wsp kernel clean`: очистка устаревших ядер.

extern crate alloc;
use alloc::format;
use libwunix::{close, open, println, t, unlink, write, O_CREAT, O_RDONLY, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::log_transaction;
use crate::display::{print_error, print_field, print_info, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};

/// Обработка подкоманд управления ядром
pub fn run_kernel_cmd(subcmd: &str) -> i32 {
    match subcmd {
        "list" | "ls" => list_kernels(),
        "rollback" | "revert" => rollback_kernel(),
        "clean" | "gc" => clean_kernels(),
        _ => {
            print_error(&format!("{} '{}'", t("неизвестная подкоманда", "unknown subcommand"), subcmd));
            println!("{}", t("Использование: wsp kernel [list | rollback | clean]", "Usage: wsp kernel [list | rollback | clean]"));
            1
        }
    }
}

/// Выводит список ядер, доступных в каталоге /boot
fn list_kernels() -> i32 {
    println!("{}", t("Ядра Wunix в каталоге /boot:", "Wunix kernels in /boot:"));

    let k_main = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if k_main >= 0 {
        close(k_main as u64);
        print_field("/boot/wunix-kernel", &format!("{}[активное / по умолчанию]{}", COLOR_GREEN, COLOR_RESET));
    } else {
        print_field("/boot/wunix-kernel", &format!("{}[отсутствует!]{}", COLOR_RED, COLOR_RESET));
    }

    let k_old = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if k_old >= 0 {
        close(k_old as u64);
        print_field("/boot/wunix-kernel.old", &format!("{}[резервное / откат]{}", COLOR_YELLOW, COLOR_RESET));
    } else {
        print_field("/boot/wunix-kernel.old", t("[не создано]", "[not created]"));
    }

    0
}

/// Выполняет откат на резервное ядро в limine.conf
fn rollback_kernel() -> i32 {
    print_info(t(
        "активация резервного ядра в конфигурации Limine...",
        "activating fallback kernel in Limine configuration...",
    ));

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
        println!("{}", t("wsp: резервное ядро назначено загрузочным по умолчанию.", "wsp: fallback kernel is now set as the default boot kernel."));
        println!("{}", t("Для запуска резервного ядра выполните перезагрузку ('reboot').", "To boot the fallback kernel, restart the machine ('reboot')."));
        0
    } else {
        print_error(t("ошибка записи в /boot/limine.conf", "failed to write to /boot/limine.conf"));
        1
    }
}

/// Удаляет устаревшие резервные ядра для экономии дискового пространства
fn clean_kernels() -> i32 {
    print_info(t("очистка резервных ядер...", "cleaning fallback kernels..."));
    let _ = unlink(b"/boot/wunix-kernel.old\0");
    println!("{}", t("wsp: каталог /boot очищен от резервных копий.", "wsp: /boot cleaned of fallback copies."));
    0
}
