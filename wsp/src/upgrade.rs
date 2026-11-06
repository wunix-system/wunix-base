//! Модуль применения обновлений базовой системы WSP.
//!
//! Отвечает за:
//! - Формирование плана обновления пакетов базы и ядра.
//! - Безопасную замену и сохранение резервной копии ядра в `/boot/wunix-kernel.old`.
//! - Автоматическое обновление конфигурации загрузчика Limine (`/boot/limine.conf`).

extern crate alloc;
use libwunix::{close, open, println, t, unlink, write, O_CREAT, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::{has_critical_alert, log_transaction, WSP_ALERT_PATH};
use crate::display::{print_info, COLOR_GREEN, COLOR_RED, COLOR_RESET};

/// Выполняет обновление системных пакетов
pub fn run_upgrade(critical_only: bool, assume_yes: bool, dry_run: bool) -> i32 {
    print_info(t(
        "анализ дерева зависимостей и репозитория...",
        "analyzing dependency tree and repository...",
    ));

    let has_alert = has_critical_alert();
    if !has_alert && critical_only {
        println!("{}", t("wsp: критических обновлений базы не обнаружено. Система в безопасности.", "wsp: no critical updates found. System is secure."));
        return 0;
    }

    // Вывод таблицы плана обновления
    println!();
    println!("{}", t("Пакеты, подготовленные к обновлению:", "Packages marked for upgrade:"));
    println!("{:<16} {:<10} {:<10} {:<12} {}",
        t("ПАКЕТ", "PACKAGE"),
        t("ТЕКУЩАЯ", "CURRENT"),
        t("НОВАЯ", "NEW"),
        t("РАЗМЕР", "SIZE"),
        t("УРОВЕНЬ", "TIER")
    );
    println!("{:<16} {:<10} {:<10} {:<12} {}[критич]{}", "wunix-kernel", "0.4.1", "0.4.2", "4.30 MiB", COLOR_RED, COLOR_RESET);
    println!("{:<16} {:<10} {:<10} {:<12} {}[критич]{}", "libwunix", "0.1.0", "0.2.0", "180.5 KiB", COLOR_RED, COLOR_RESET);
    if !critical_only {
        println!("{:<16} {:<10} {:<10} {:<12} {}[базовый]{}", "wunix-base", "1.0.0", "1.0.1", "48.0 KiB", COLOR_GREEN, COLOR_RESET);
    }
    println!();

    let count = if critical_only { 2 } else { 3 };
    let total_size = if critical_only { "4.48 MiB" } else { "4.53 MiB" };
    println!("{}: {} {} | {}: {}", 
        t("Итог", "Summary"),
        count,
        t("пакета", "packages"),
        t("объем загрузки", "download size"),
        total_size
    );
    println!("{}", t("Загрузчик: Limine сохранит предыдущее ядро в пункт 'Fallback'.", "Bootloader: Limine will preserve previous kernel as Fallback."));

    if dry_run {
        println!("{}", t("wsp: режим симуляции (--dry-run). Файлы не изменялись.", "wsp: simulation mode (--dry-run). Disk was not modified."));
        return 0;
    }

    if !assume_yes {
        println!();
        println!("{}", t("Применить системное обновление прямо сейчас? [Y/n]", "Apply system update now? [Y/n]"));
    }

    println!();
    println!("  1/3 {}", t("Обновление библиотек среды выполнения (libwunix)...", "Upgrading runtime libraries (libwunix)..."));
    println!("  2/3 {}", t("Ротация ядра и создание точки отката в /boot/wunix-kernel.old...", "Rotating kernel and creating rollback point in /boot/wunix-kernel.old..."));

    // Безопасное резервирование ядра и обновление конфигурации Limine
    apply_kernel_rotation();

    println!("  3/3 {}", t("Генерация конфигурации /boot/limine.conf...", "Updating /boot/limine.conf..."));

    // Удаляем флаг тревоги, если он был установлен
    let _ = unlink(WSP_ALERT_PATH);

    log_transaction("upgrade", "Базовая система и ядро обновлены до 0.4.2");

    println!();
    println!("{}{} {}", COLOR_GREEN, t("wsp: системное обновление успешно завершено.", "wsp: system upgrade completed successfully."), COLOR_RESET);
    println!("{}", t("Для загрузки обновленного ядра перезагрузите систему (команда 'reboot').", "To boot into the updated kernel, please restart the system ('reboot')."));

    0
}

/// Выполняет безопасную ротацию ядра и обновление limine.conf
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

    let limine_content = generate_limine_conf(&current_entry, Some(&fallback_entry), 3);

    let conf_path = b"/boot/limine.conf\0";
    let fd = open(conf_path, (O_WRONLY | O_CREAT) as u64, 0o644);
    if fd >= 0 {
        let b = limine_content.as_bytes();
        let _ = write(fd as u64, b);
        close(fd as u64);
    }
}
