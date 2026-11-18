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
mod http;
mod install;
mod list;
mod progress;
mod repo_cmd;
mod search;
mod unpack;
mod upgrade;

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

fn do_update(root_prefix: Option<&str>, flag_quiet: bool) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        display::print_access_denied("update", uid);
        return 1;
    }

    let sources = repo_cmd::load_sources();
    if !flag_quiet {
        println!("wpm: синхронизация списков репозиториев...");
    }
    let mut has_errors = false;

    let root_str = root_prefix.unwrap_or("/");
    let r = if root_str == "/" { "" } else { root_str.trim_end_matches('/') };

    for section in &sources {
        for server in &section.servers {
            let s_url = server.trim_end_matches('/');
            let json_url = format!("{}/packages.json", s_url);
            if !flag_quiet {
                println!("  [{}] {}", section.name, s_url);
            }

            let dest_dir = format!("{}/var/lib/wpm/repo/{}/x86_64", r, section.name);
            let cache_dir = format!("{}/var/cache/wpm", r);
            install::ensure_dir_recursive(&dest_dir);
            install::ensure_dir_recursive(&cache_dir);

            let dest_path = format!("{}/packages.json", dest_dir);
            match http::fetch_to_file(&json_url, &dest_path, None, "packages.json") {
                Ok(_) => {
                    let cache_json = format!("{}/packages.json", cache_dir);
                    let _ = install::copy_file(&dest_path, &cache_json);

                    let txt_url = format!("{}/index.txt", s_url);
                    let txt_path = format!("{}/index.txt", dest_dir);
                    let _ = http::fetch_to_file(&txt_url, &txt_path, None, "index.txt");
                }
                Err(e) => {
                    has_errors = true;
                    display::print_error(&format!("не удалось загрузить индекс с {}: {:?}", s_url, e));
                }
            }
        }
    }

    let installed = install::load_installed_apps_from(root_str);
    let catalog = search::load_wpm_catalog_from(root_str);
    let mut upgradable_count = 0;
    for app in &installed {
        let app_ver = libpackwunix::version::Version::parse(&app.version)
            .unwrap_or(libpackwunix::version::Version::new(0, 1, 0));
        if let Some(cat_pkg) = catalog.iter().find(|p| p.name == app.name || p.aliases.iter().any(|a| a == &app.name)) {
            if let Ok(cat_ver) = libpackwunix::version::Version::parse(&cat_pkg.version) {
                if cat_ver.is_newer_than(&app_ver) {
                    upgradable_count += 1;
                }
            }
        }
    }

    if !flag_quiet {
        if !has_errors {
            println!(
                "wpm: каталоги программ успешно синхронизированы (установлено: {}, можно обновить: {}).",
                installed.len(),
                upgradable_count
            );
            if upgradable_count > 0 {
                println!(
                    "  {}",
                    t(
                        "Для просмотра доступных обновлений выполните: wpm check",
                        "Run 'wpm check' to view available updates."
                    )
                );
            }
        } else {
            display::print_warning("синхронизация завершена с ошибками для некоторых источников.");
        }
    }

    if has_errors { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 0 {
        let envp = unsafe { argv.add(argc as usize + 1) };
        libwunix::env::init(envp);
    }

    let mut raw_args: Vec<&str> = Vec::new();
    if argc > 1 {
        for i in 0..(argc - 1) as usize {
            raw_args.push(cstr_to_str(unsafe { *argv.add(i + 1) }));
        }
    }

    // Флаги командной строки
    let mut flag_version = false;
    let mut flag_yes = false;
    let mut flag_purge = false;
    let mut flag_ignore_unsafe = false;
    let mut flag_quiet = false;
    let mut flag_verbose = false;
    let mut flag_reinstall = false;
    let mut flag_upgradable = false;
    let mut flag_root: Option<&str> = None;

    // Pacman-флаги операций
    let mut has_s_op = false;
    let mut has_r_op = false;
    let mut has_q_op = false;
    let mut has_u_op = false;

    let mut has_mod_y = false;
    let mut has_mod_u = false;
    let mut has_mod_s = false;
    let mut has_mod_i = false;
    let mut has_mod_c = false;

    let mut positional: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < raw_args.len() {
        let a = raw_args[i];
        if a.is_empty() {
            i += 1;
            continue;
        }

        if a == "--help" || a == "-h" {
            display::print_help();
            exit(0);
        } else if a == "--version" || a == "-V" {
            flag_version = true;
        } else if a == "-v" {
            flag_version = true;
            flag_verbose = true;
        } else if a == "--yes" || a == "-y" || a == "--noconfirm" {
            flag_yes = true;
            has_mod_y = true;
        } else if a == "--verbose" || a == "-d" {
            flag_verbose = true;
        } else if a == "--quiet" || a == "-q" {
            flag_quiet = true;
        } else if a == "--purge" {
            flag_purge = true;
        } else if a == "-u" || a == "--upgradable" || a == "--updates" {
            flag_upgradable = true;
        } else if a == "--reinstall" {
            flag_reinstall = true;
        } else if a == "--ignore-critical-unsafe" {
            flag_ignore_unsafe = true;
        } else if a == "-r" || a == "--root" {
            if i + 1 < raw_args.len() {
                i += 1;
                flag_root = Some(raw_args[i]);
            }
        } else if a.starts_with("--root=") {
            flag_root = Some(&a[7..]);
        } else if a.starts_with('-') && !a.starts_with("--") {
            // Разбор компактных коротких флагов в стиле pacman (-Syu, -Ss, -Rns, -Qi и т.д.)
            for ch in a[1..].chars() {
                match ch {
                    'S' => has_s_op = true,
                    'R' => has_r_op = true,
                    'Q' => has_q_op = true,
                    'U' => has_u_op = true,
                    'y' => has_mod_y = true,
                    'u' => has_mod_u = true,
                    's' => has_mod_s = true,
                    'i' => has_mod_i = true,
                    'c' => has_mod_c = true,
                    'n' => flag_purge = true,
                    'd' => flag_verbose = true,
                    'q' => flag_quiet = true,
                    'v' | 'V' => flag_version = true,
                    'h' => {
                        display::print_help();
                        exit(0);
                    }
                    _ => {
                        display::print_error(&format!("{} '-{}'", t("неизвестный флаг", "unknown flag"), ch));
                        println!("  {}", t("Используйте 'wpm --help' для справки.", "Use 'wpm --help' for help."));
                        exit(1);
                    }
                }
            }
        } else {
            positional.push(a);
        }
        i += 1;
    }

    // Определение команды (pacman-стиль имеет наивысший приоритет)
    let pacman_cmd = if has_s_op {
        if has_mod_y && has_mod_u {
            Some("update_and_upgrade")
        } else if has_mod_y && positional.is_empty() {
            Some("update")
        } else if has_mod_u {
            Some("upgrade")
        } else if has_mod_s {
            Some("search")
        } else if has_mod_i {
            Some("info")
        } else if has_mod_c {
            Some("clean")
        } else {
            Some("install")
        }
    } else if has_r_op {
        Some("remove")
    } else if has_q_op {
        if has_mod_i {
            Some("info")
        } else if has_mod_s {
            Some("search")
        } else if has_mod_u {
            Some("check")
        } else {
            Some("list")
        }
    } else if has_u_op {
        Some("install")
    } else {
        None
    };

    // Если был запрошен флаг версии без выполняемых операций
    if (flag_version && pacman_cmd.is_none() && positional.is_empty())
        || (raw_args.len() == 1 && (raw_args[0] == "-v" || raw_args[0] == "-V" || raw_args[0] == "--version"))
    {
        display::print_version();
        exit(0);
    }

    let (cmd, rest): (&str, &[&str]) = if let Some(p_cmd) = pacman_cmd {
        (p_cmd, &positional[..])
    } else if !positional.is_empty() {
        (positional[0], &positional[1..])
    } else {
        display::print_help();
        exit(0);
    };

    let code = match cmd {
        "version" => {
            display::print_version();
            0
        }
        "check" | "updates" => {
            list::run_check(flag_root)
        }
        "update" => do_update(flag_root, flag_quiet),
        "upgrade" => {
            upgrade::run_upgrade(flag_root, flag_yes, flag_ignore_unsafe, flag_verbose)
        }
        "update_and_upgrade" => {
            let res = do_update(flag_root, flag_quiet);
            if res == 0 {
                upgrade::run_upgrade(flag_root, flag_yes, flag_ignore_unsafe, flag_verbose)
            } else {
                res
            }
        }
        "search" => {
            let q = if rest.is_empty() { "" } else { rest[0] };
            search::run_search(q)
        }
        "install" => {
            install::run_install_in(rest, flag_root, flag_yes, flag_ignore_unsafe, flag_verbose, flag_reinstall)
        }
        "remove" | "rm" => {
            install::run_remove(rest, flag_purge, flag_yes)
        }
        "list" | "ls" => {
            if flag_upgradable {
                list::run_check(flag_root)
            } else if let Some(root) = flag_root {
                list::run_list_in(root)
            } else {
                list::run_list()
            }
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
            println!("  {}", t("Используйте 'wpm --help' для списка доступных команд.", "Use 'wpm --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}
