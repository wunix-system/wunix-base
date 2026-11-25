#![no_std]
#![no_main]

//! Единый пакетный менеджер Wunix Package Manager (WPM).
//!
//! Управляет как системными пакетами (ядро, библиотеки, базовые утилиты
//! из неизменяемого источника [system]), так и прикладными программами
//! и метапакетами (.wp).

extern crate alloc;

mod allocator;
mod audit;
mod db;
mod display;
mod guard;
mod http;
mod install;
mod kernel_cmd;
mod list;
mod progress;
mod repo_cmd;
mod resolver;
mod search;
mod unpack;
mod upgrade;

use alloc::format;
use alloc::vec::Vec;
use libwunix::{exit, geteuid, getuid, println, t};
use libpackwunix::sources::get_wsp_system_repo;
use libpackwunix::version::Version;

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

    let root_str = root_prefix.unwrap_or("/");
    if !flag_quiet {
        println!("wpm: синхронизация списков репозиториев...");
    }

    let mut has_errors = false;

    // 1. Обязательная синхронизация неизменяемого системного канала [system]
    let sys_url = get_wsp_system_repo().trim_end_matches('/');
    if !flag_quiet {
        println!("  [system] {}", sys_url);
    }
    let sys_dir = db::get_path(root_str, "/var/lib/wpm/repo/system/x86_64");
    db::ensure_dir_recursive(&sys_dir);
    let sys_json_url = format!("{}/packages.json", sys_url);
    let sys_json_dst = format!("{}/packages.json", sys_dir);
    if let Err(e) = http::fetch_to_file(&sys_json_url, &sys_json_dst, None, "system/packages.json") {
        has_errors = true;
        display::print_error(&format!("не удалось загрузить системный индекс {}: {:?}", sys_url, e));
    } else {
        let _ = http::fetch_to_file(
            &format!("{}/index.txt", sys_url),
            &format!("{}/index.txt", sys_dir),
            None,
            "system/index.txt",
        );
    }

    // 2. Синхронизация пользовательских репозиториев из /etc/wpm/sources.conf
    let sources = repo_cmd::load_sources();
    for section in &sources {
        for server in &section.servers {
            let s_url = server.trim_end_matches('/');
            if !flag_quiet {
                println!("  [{}] {}", section.name, s_url);
            }
            let dest_dir = db::get_path(root_str, &format!("/var/lib/wpm/repo/{}/x86_64", section.name));
            db::ensure_dir_recursive(&dest_dir);

            let dest_path = format!("{}/packages.json", dest_dir);
            match http::fetch_to_file(&format!("{}/packages.json", s_url), &dest_path, None, "packages.json") {
                Ok(_) => {
                    let _ = http::fetch_to_file(
                        &format!("{}/index.txt", s_url),
                        &format!("{}/index.txt", dest_dir),
                        None,
                        "index.txt",
                    );
                }
                Err(e) => {
                    has_errors = true;
                    display::print_error(&format!("не удалось загрузить индекс с {}: {:?}", s_url, e));
                }
            }
        }
    }

    let installed = db::load_installed_packages_from(root_str);
    let catalog = db::load_unified_catalog_from(root_str);
    let mut upgradable_count = 0;
    for inst in &installed {
        let inst_ver = Version::parse(&inst.version).unwrap_or(Version::new(0, 1, 0));
        if let Some(entry) = catalog.iter().find(|c| c.pkg.name == inst.name) {
            if entry.pkg.version.is_newer_than(&inst_ver) {
                upgradable_count += 1;
            }
        }
    }

    if !flag_quiet {
        if !has_errors {
            println!(
                "wpm: каталоги успешно синхронизированы (установлено: {}, можно обновить: {}).",
                installed.len(),
                upgradable_count
            );
            if upgradable_count > 0 {
                println!("  {}", t("Для просмотра обновлений выполните: wpm check", "Run 'wpm check' to view updates."));
            }
        } else {
            display::print_warning("синхронизация завершена с ошибками для некоторых источников.");
        }
    }

    if has_errors { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    libwunix::init(argc, argv);

    let mut raw_args: Vec<&str> = Vec::new();
    if argc > 1 {
        for i in 0..(argc - 1) as usize {
            raw_args.push(cstr_to_str(unsafe { *argv.add(i + 1) }));
        }
    }

    let mut flag_version = false;
    let mut flag_yes = false;
    let mut flag_purge = false;
    let mut flag_ignore_unsafe = false;
    let mut flag_quiet = false;
    let mut flag_verbose = false;
    let mut flag_reinstall = false;
    let mut flag_upgradable = false;
    let mut flag_critical = false;
    let mut flag_dry_run = false;
    let mut flag_json = false;
    let mut flag_files = false;
    let mut flag_root: Option<&str> = None;

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
        } else if a == "--verbose" || a == "-d" || a == "--verify" || a == "--verify-files" {
            flag_verbose = true;
        } else if a == "--quiet" || a == "-q" {
            flag_quiet = true;
        } else if a == "--purge" {
            flag_purge = true;
        } else if a == "-u" || a == "--upgradable" || a == "--updates" {
            flag_upgradable = true;
        } else if a == "-c" || a == "--critical" || a == "--critical-only" {
            flag_critical = true;
        } else if a == "--dry-run" {
            flag_dry_run = true;
        } else if a == "--json" {
            flag_json = true;
        } else if a == "-f" || a == "--files" {
            flag_files = true;
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
                    'c' => {
                        has_mod_c = true;
                        flag_critical = true;
                    }
                    'n' => flag_purge = true,
                    'd' => flag_verbose = true,
                    'q' => flag_quiet = true,
                    'f' => flag_files = true,
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
        } else if has_mod_c && positional.is_empty() {
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
        "audit" => audit::run_audit(flag_verbose),
        "check" | "updates" => audit::run_check(flag_root, flag_critical, flag_json),
        "update" => do_update(flag_root, flag_quiet),
        "upgrade" => upgrade::run_upgrade(flag_root, flag_critical, flag_yes, flag_ignore_unsafe, flag_dry_run, flag_verbose),
        "update_and_upgrade" => {
            let res = do_update(flag_root, flag_quiet);
            if res == 0 {
                upgrade::run_upgrade(flag_root, flag_critical, flag_yes, flag_ignore_unsafe, flag_dry_run, flag_verbose)
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
        "reinstall" => {
            install::run_install_in(rest, flag_root, flag_yes, flag_ignore_unsafe, flag_verbose, true)
        }
        "remove" | "rm" => install::run_remove(rest, flag_purge, flag_yes),
        "list" | "ls" => {
            if flag_upgradable {
                audit::run_check(flag_root, flag_critical, flag_json)
            } else {
                list::run_list_in(flag_root.unwrap_or("/"), flag_critical)
            }
        }
        "info" => {
            if let Some(&name) = rest.first() {
                list::run_info(name, flag_files)
            } else {
                display::print_error(t("укажите имя пакета: wpm info <пакет>", "missing package name: wpm info <package>"));
                1
            }
        }
        "repo" => repo_cmd::run_repo_cmd(rest),
        "kernel" => {
            let sub = rest.first().copied().unwrap_or("list");
            kernel_cmd::run_kernel_cmd(sub)
        }
        "rollback" => {
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                display::print_access_denied("rollback", uid);
                1
            } else {
                kernel_cmd::run_kernel_cmd("rollback")
            }
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
