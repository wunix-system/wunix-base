#![no_std]
#![no_main]

//! Системный менеджер пакетов Wunix System Package (WSP).
//!
//! Отвечает за управление базовой системой: ядром, системными библиотеками
//! и безопасными A/B обновлениями загрузчика.

extern crate alloc;

mod allocator;
mod audit;
mod db;
mod display;
mod kernel_cmd;
mod list;
mod upgrade;

use libwunix::{exit, println, t};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}

fn cstr_to_str(ptr: *const u8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).unwrap_or("")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 0 {
        let envp = unsafe { argv.add(argc as usize + 1) };
        libwunix::env::init(envp);
    }

    let mut args: [&str; 16] = [""; 16];
    let count = if argc > 1 {
        let n = ((argc - 1) as usize).min(16);
        for i in 0..n {
            args[i] = cstr_to_str(unsafe { *argv.add(i + 1) });
        }
        n
    } else {
        0
    };

    if count == 0 {
        display::print_help();
        exit(0);
    }

    let cmd = args[0];

    // Флаги
    let mut flag_critical = false;
    let mut flag_yes = false;
    let mut flag_verify = false;
    let mut flag_dry_run = false;
    let mut flag_json = false;
    let mut flag_files = false;
    let mut extra_arg: Option<&str> = None;

    let mut i = 1;
    while i < count {
        let a = args[i];
        match a {
            "-h" | "--help" => {
                display::print_help();
                exit(0);
            }
            "-c" | "--critical" | "--critical-only" => flag_critical = true,
            "-y" | "--yes" => flag_yes = true,
            "-v" | "--verify" | "--verify-files" => flag_verify = true,
            "--dry-run" => flag_dry_run = true,
            "--json" => flag_json = true,
            "-f" | "--files" => flag_files = true,
            _ => {
                if extra_arg.is_none() && !a.starts_with('-') {
                    extra_arg = Some(a);
                }
            }
        }
        i += 1;
    }

    let code = match cmd {
        "audit" => audit::run_audit(flag_verify),
        "check" => audit::run_check(flag_critical, flag_json),
        "update" => {
            println!("{}[WSP]{} Индекс системного репозитория синхронизирован (OK).", display::COLOR_CYAN, display::COLOR_RESET);
            0
        }
        "upgrade" => upgrade::run_upgrade(flag_critical, flag_yes, flag_dry_run),
        "list" | "ls" => list::run_list(false, flag_critical),
        "info" => {
            if let Some(pkg) = extra_arg {
                list::run_info(pkg, flag_files)
            } else {
                println!("{}[!] Укажите имя пакета: wsp info <пакет>{}", display::COLOR_RED, display::COLOR_RESET);
                1
            }
        }
        "kernel" => {
            let sub = extra_arg.unwrap_or("list");
            kernel_cmd::run_kernel_cmd(sub)
        }
        "rollback" => {
            println!("{}[WSP]{} Выполняется откат последней системной транзакции...", display::COLOR_CYAN, display::COLOR_RESET);
            kernel_cmd::run_kernel_cmd("rollback")
        }
        "-h" | "--help" => {
            display::print_help();
            0
        }
        _ => {
            println!("{}[!] Неизвестная команда WSP: '{}'{}", display::COLOR_RED, cmd, display::COLOR_RESET);
            println!("{}", t("Используйте 'wsp --help' для списка доступных команд.", "Use 'wsp --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}
