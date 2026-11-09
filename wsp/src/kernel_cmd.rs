//! Модуль управления ядрами WSP (`wsp kernel`).

extern crate alloc;
use alloc::format;
use libwunix::{close, geteuid, getuid, open, println, t, unlink, write, O_CREAT, O_RDONLY, O_WRONLY};
use libpackwunix::kernel::{generate_limine_conf, KernelEntry};
use libpackwunix::version::Version;

use crate::db::log_transaction;
use crate::display::{print_access_denied, print_error, COLOR_BOLD, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};

pub fn run_kernel_cmd(subcmd: &str) -> i32 {
    match subcmd {
        "list" | "ls" => list_kernels(),
        "rollback" | "revert" => {
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                print_access_denied("kernel rollback", uid);
                1
            } else {
                rollback_kernel()
            }
        }
        "clean" | "gc" => {
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                print_access_denied("kernel clean", uid);
                1
            } else {
                clean_kernels()
            }
        }
        _ => {
            print_error(&format!("{} '{}'", t("неизвестная подкоманда", "unknown subcommand"), subcmd));
            println!("  {}", t("Использование: wsp kernel [list | rollback | clean]", "Usage: wsp kernel [list | rollback | clean]"));
            1
        }
    }
}

fn list_kernels() -> i32 {
    println!("Ядра в /boot:");

    let k_main = open(b"/boot/wunix-kernel\0", O_RDONLY as u64, 0);
    if k_main >= 0 {
        close(k_main as u64);
        println!("  /boot/wunix-kernel       {}{}{}", COLOR_GREEN, t("[активное ядро]", "[active]"), COLOR_RESET);
    } else {
        println!("  /boot/wunix-kernel       {}{}{}", COLOR_RED, t("[отсутствует]", "[missing]"), COLOR_RESET);
    }

    let k_old = open(b"/boot/wunix-kernel.old\0", O_RDONLY as u64, 0);
    if k_old >= 0 {
        close(k_old as u64);
        println!("  /boot/wunix-kernel.old   {}{}{}", COLOR_YELLOW, t("[резервное ядро]", "[fallback]"), COLOR_RESET);
    } else {
        println!("  /boot/wunix-kernel.old   {}", t("[резервное ядро не создано]", "[no fallback]"));
    }

    0
}

fn rollback_kernel() -> i32 {
    println!("wsp: переключение загрузчика на резервное ядро...");

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
        println!("wsp: резервное ядро /boot/wunix-kernel.old назначено загрузочным.");
        println!("  {}: {}reboot{}", t("Перезагрузите систему для применения", "Reboot to apply"), COLOR_BOLD, COLOR_RESET);
        0
    } else {
        print_error(t("ошибка записи в /boot/limine.conf", "failed to write to /boot/limine.conf"));
        1
    }
}

fn clean_kernels() -> i32 {
    let _ = unlink(b"/boot/wunix-kernel.old\0");
    println!("wsp: резервные копии ядра в /boot очищены.");
    0
}
