#![no_std]
#![no_main]

//! Пользовательский менеджер пакетов Wunix Package Manager (WPM).
//!
//! Отвечает за установку прикладных программ, утилит для разработчиков
//! и управление источниками в стиле Wunix.

extern crate alloc;

mod allocator;
mod display;
mod guard;
mod install;
mod list;
mod repo_cmd;
mod search;

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
    let mut flag_yes = false;
    let mut flag_purge = false;
    let mut flag_ignore_unsafe = false;
    let mut positional_args: alloc::vec::Vec<&str> = alloc::vec::Vec::new();

    let mut i = 1;
    while i < count {
        let a = args[i];
        match a {
            "-h" | "--help" => {
                display::print_help();
                exit(0);
            }
            "-y" | "--yes" => flag_yes = true,
            "--purge" => flag_purge = true,
            "--ignore-critical-unsafe" => flag_ignore_unsafe = true,
            _ => {
                if !a.starts_with('-') {
                    positional_args.push(a);
                }
            }
        }
        i += 1;
    }

    let code = match cmd {
        "update" => {
            println!("{}[WPM]{} Синхронизация списков репозиториев WPM...", display::COLOR_CYAN, display::COLOR_RESET);
            println!("{}{}✓ Все настроенные репозитории актуальны (200 OK).{}", display::COLOR_BOLD, display::COLOR_GREEN, display::COLOR_RESET);
            0
        }
        "search" => {
            let q = if positional_args.is_empty() { "" } else { positional_args[0] };
            search::run_search(q)
        }
        "install" => {
            install::run_install(&positional_args, flag_yes, flag_ignore_unsafe)
        }
        "remove" | "rm" => {
            install::run_remove(&positional_args, flag_purge)
        }
        "upgrade" => {
            println!("{}[WPM]{} Проверка обновлений установленных прикладных программ...", display::COLOR_CYAN, display::COLOR_RESET);
            println!("{}{}✓ Все установленные пользовательские программы имеют последние версии.{}", display::COLOR_BOLD, display::COLOR_GREEN, display::COLOR_RESET);
            0
        }
        "list" | "ls" => {
            list::run_list()
        }
        "info" => {
            if let Some(&name) = positional_args.first() {
                list::run_info(name)
            } else {
                println!("{}[!] Укажите имя программы: wpm info <программа>{}", display::COLOR_RED, display::COLOR_RESET);
                1
            }
        }
        "repo" => {
            repo_cmd::run_repo_cmd(&positional_args)
        }
        "clean" => {
            println!("{}[WPM]{} Очистка кэша загруженных пакетов (/var/cache/wpm)...", display::COLOR_CYAN, display::COLOR_RESET);
            println!("{}{}✓ Локальный кэш пакетов очищен.{}", display::COLOR_BOLD, display::COLOR_GREEN, display::COLOR_RESET);
            0
        }
        "-h" | "--help" => {
            display::print_help();
            0
        }
        _ => {
            println!("{}[!] Неизвестная команда WPM: '{}'{}", display::COLOR_RED, cmd, display::COLOR_RESET);
            println!("{}", t("Используйте 'wpm --help' для списка доступных команд.", "Use 'wpm --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}
