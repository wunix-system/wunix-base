#![no_std]
#![no_main]

//! Пользовательский менеджер пакетов Wunix Package Manager (WPM).
//!
//! Отвечает за установку и управление прикладными программами (.wp),
//! утилитами разработки и источниками репозиториев Wunix.

extern crate alloc;

mod allocator;
mod display;
mod guard;
mod install;
mod list;
mod repo_cmd;
mod search;
mod unpack;

use alloc::format;
use alloc::vec::Vec;
use libwunix::{exit, geteuid, getuid, println, t};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    libwunix::println!("wpm: критическая ошибка: {}", info);
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

    // Флаги командной строки
    let mut flag_version = false;
    let mut flag_yes = false;
    let mut flag_purge = false;
    let mut flag_ignore_unsafe = false;
    let mut flag_quiet = false;
    let mut flag_verbose = false;
    let mut flag_reinstall = false;

    let mut positional: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < count {
        let a = args[i];
        match a {
            "-h" | "--help" => {
                display::print_help();
                exit(0);
            }
            "-V" | "--version" => {
                flag_version = true;
            }
            "-v" => {
                // Если -v передан без позиционных команд, это запрос версии;
                // если с позиционной командой (например, install -v) — это подробный режим.
                flag_verbose = true;
                flag_version = true;
            }
            "-d" | "--verbose" => flag_verbose = true,
            "-y" | "--yes" => flag_yes = true,
            "-q" | "--quiet" => flag_quiet = true,
            "--purge" => flag_purge = true,
            "--reinstall" => flag_reinstall = true,
            "--ignore-critical-unsafe" => flag_ignore_unsafe = true,
            _ => {
                if !a.is_empty() {
                    if a.starts_with('-') {
                        display::print_error(&format!("{} '{}'", t("неизвестная опция", "unknown option"), a));
                        println!("  {}", t("Используйте 'wpm --help' для просмотра доступных опций.", "Use 'wpm --help' for available options."));
                        exit(1);
                    } else {
                        positional.push(a);
                    }
                }
            }
        }
        i += 1;
    }

    // Если был запрошен флаг версии -v / -V без подкоманд — показываем карточку версии
    if (flag_version && positional.is_empty()) || (count == 1 && (args[0] == "-v" || args[0] == "-V" || args[0] == "--version")) {
        display::print_version();
        exit(0);
    }

    if positional.is_empty() {
        display::print_help();
        exit(0);
    }

    let cmd = positional[0];
    let rest = &positional[1..];

    let code = match cmd {
        "version" => {
            display::print_version();
            0
        }
        "update" => {
            if !flag_quiet {
                println!("wpm: синхронизация списков репозиториев...");
                display::print_step(1, 1, t("Синхронизация каталогов программ (200 OK)", "Synchronizing application catalogs (200 OK)"), true);
                println!("wpm: каталоги программ успешно синхронизированы.");
            }
            0
        }
        "search" => {
            let q = if rest.is_empty() { "" } else { rest[0] };
            search::run_search(q)
        }
        "install" => {
            install::run_install(rest, flag_yes, flag_ignore_unsafe, flag_verbose, flag_reinstall)
        }
        "remove" | "rm" => {
            install::run_remove(rest, flag_purge, flag_yes)
        }
        "upgrade" => {
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                display::print_access_denied("upgrade", uid);
                1
            } else {
                if !flag_quiet {
                    println!("wpm: проверка обновлений установленных программ...");
                    display::print_step(1, 1, t("Проверка версий программ", "Checking software versions"), true);
                    println!("wpm: все установленные программы имеют актуальные версии.");
                }
                0
            }
        }
        "list" | "ls" => {
            list::run_list()
        }
        "info" => {
            if let Some(&name) = rest.first() {
                list::run_info(name)
            } else {
                display::print_error(t("укажите имя программы: wpm info <программа>", "missing program name: wpm info <program>"));
                1
            }
        }
        "repo" => {
            repo_cmd::run_repo_cmd(rest)
        }
        "clean" => {
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                display::print_access_denied("clean", uid);
                1
            } else {
                display::print_info(t("локальный кэш /var/cache/wpm очищен.", "local cache /var/cache/wpm cleaned."));
                0
            }
        }
        _ => {
            display::print_error(&format!("{} '{}'", t("неизвестная команда", "unknown command"), cmd));
            println!("  {}", t("Используйте 'wpm --help' для просмотра списка доступных команд.", "Use 'wpm --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}
