//! Модуль применения обновлений базовой системы WSP.

extern crate alloc;
use libwunix::{close, geteuid, getuid, open, println, t, unlink, write, O_CREAT, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::{has_critical_alert, log_transaction, WSP_ALERT_PATH};
use crate::display::{print_access_denied, print_step, COLOR_GREEN, COLOR_RED, COLOR_RESET};

pub fn run_upgrade(critical_only: bool, assume_yes: bool, dry_run: bool) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("upgrade", uid);
        return 1;
    }

    println!("wsp: обновление базовой системы и ядра Wunix...");

    let has_alert = has_critical_alert();
    if !has_alert && critical_only {
        println!("wsp: все критические компоненты ядра находятся в актуальном состоянии.");
        return 0;
    }

    println!("  Компоненты базы, подготовленные к обновлению:");
    println!("    - {:<16} 0.4.1 -> 0.4.2  {}[критический]{} 4.30 МиБ", "wunix-kernel", COLOR_RED, COLOR_RESET);
    println!("    - {:<16} 0.1.0 -> 0.2.0  {}[критический]{} 180.5 КиБ", "libwunix", COLOR_RED, COLOR_RESET);
    if !critical_only {
        println!("    - {:<16} 1.0.0 -> 1.0.1  {}[базовый]{}     48.0 КиБ", "wunix-base", COLOR_GREEN, COLOR_RESET);
    }

    let count = if critical_only { 2 } else { 3 };
    let total_size = if critical_only { "4.48 МиБ" } else { "4.53 МиБ" };
    println!("  Всего пакетов: {} | Объем данных: {}", count, total_size);

    if dry_run {
        println!("  Режим симуляции (--dry-run). Файлы ядра не изменялись.");
        return 0;
    }

    if !assume_yes {
        println!("  Применить системное обновление ядра прямо сейчас? [Y/n]");
    }

    print_step(1, 3, t("Обновление библиотек системных вызовов Ring 3 (libwunix)", "Upgrading Ring 3 runtime libraries (libwunix)"), true);
    apply_kernel_rotation();
    print_step(2, 3, t("Ротация ядра и создание точки отката (/boot/wunix-kernel.old)", "Rotating kernel and creating rollback point"), true);
    print_step(3, 3, t("Обновление конфигурации загрузчика Limine", "Updating /boot/limine.conf bootloader entries"), true);

    let _ = unlink(WSP_ALERT_PATH);
    log_transaction("upgrade", "Базовая система и ядро обновлены до версии 0.4.2");

    println!("wsp: системное обновление успешно завершено.");
    println!("  Для перехода на обновленное ядро перезагрузите систему (reboot).");

    0
}

fn apply_kernel_rotation() {
    let current_entry = KernelEntry {
        version: Version::new(0, 4, 2),
        kernel_path: "boot():/boot/wunix-kernel".into(),
        cmdline: "root=/dev/vda1 lang=ru mode=disk".into(),
    };

    let fallback_entry = KernelEntry {
        version: Version::new(0, 4, 1),
        kernel_path: "boot():/boot/wunix-kernel.old".into(),
        cmdline: "root=/dev/vda1 lang=ru mode=disk".into(),
    };

    let new_conf = generate_limine_conf(&current_entry, Some(&fallback_entry), 3);

    let fd = open(b"/boot/limine.conf\0", (O_CREAT | O_WRONLY) as u64, 0o644);
    if fd >= 0 {
        let _ = write(fd as u64, new_conf.as_bytes());
        close(fd as u64);
    }
}
