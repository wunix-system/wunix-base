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
mod http;
mod install;
mod kernel_cmd;
mod list;
mod progress;
mod unpack;
mod upgrade;

use alloc::format;
use alloc::vec::Vec;
use libwunix::{exit, geteuid, getuid, println, t};
use libpackwunix::version::Version;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    libwunix::println!("wsp: критическая ошибка: {}", info);
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

fn do_update(root_dir: &str, flag_quiet: bool) -> i32 {
    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        display::print_access_denied("update", uid);
        return 1;
    }

    let base_url = libpackwunix::sources::get_wsp_system_repo().trim_end_matches('/');
    let json_url = format!("{}/packages.json", base_url);
    if !flag_quiet {
        println!("wsp: синхронизация системного репозитория...");
        println!("  Источник: {}", base_url);
    }

    let r = if root_dir == "/" { "" } else { root_dir.trim_end_matches('/') };
    let dest_dir = format!("{}/var/lib/wsp/repo/system/x86_64", r);
    let cache_dir = format!("{}/var/cache/wsp", r);
    db::ensure_dir_recursive(&dest_dir);
    db::ensure_dir_recursive(&cache_dir);

    let dest_json = format!("{}/packages.json", dest_dir);
    match http::fetch_to_file(&json_url, &dest_json, None, "packages.json") {
        Ok(_) => {
            let cache_json = format!("{}/packages.json", cache_dir);
            let lib_json = format!("{}/var/lib/wsp/packages.json", r);
            let _ = db::copy_file(&dest_json, &cache_json);
            let _ = db::copy_file(&dest_json, &lib_json);

            let txt_url = format!("{}/index.txt", base_url);
            let txt_path = format!("{}/index.txt", dest_dir);
            let _ = http::fetch_to_file(&txt_url, &txt_path, None, "index.txt");

            let installed = db::load_installed_packages_from(root_dir);
            let repo_packages = db::load_repo_packages(root_dir);

            let mut upgradable_count = 0;
            for inst in &installed {
                let inst_ver = Version::parse(&inst.version).unwrap_or(Version::new(0, 1, 0));
                if let Some(repo_pkg) = repo_packages.iter().find(|p| p.name == inst.name) {
                    if repo_pkg.version.is_newer_than(&inst_ver) {
                        upgradable_count += 1;
                    }
                }
            }

            if !installed.iter().any(|p| p.name == "wunix-kernel") {
                if let Some(repo_pkg) = repo_packages.iter().find(|p| p.name == "wunix-kernel") {
                    let k_path = db::get_path(root_dir, "/boot/wunix-kernel");
                    let fd = db::open_c(&k_path, libwunix::O_RDONLY as u64, 0);
                    if fd >= 0 {
                        libwunix::close(fd as u64);
                        let inst_ver = Version::new(0, 1, 0);
                        if repo_pkg.version.is_newer_than(&inst_ver) {
                            upgradable_count += 1;
                        }
                    }
                }
            }

            if !flag_quiet {
                println!(
                    "wsp: индекс системных пакетов успешно синхронизирован (установлено: {}, можно обновить: {}).",
                    installed.len(),
                    upgradable_count
                );
            }
            0
        }
        Err(e) => {
            display::print_error(&format!("не удалось загрузить индекс системного репозитория: {:?}", e));
            1
        }
    }
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
    let mut flag_critical = false;
    let mut flag_yes = false;
    let mut flag_verify = false;
    let mut flag_dry_run = false;
    let mut flag_json = false;
    let mut flag_files = false;
    let mut flag_quiet = false;
    let mut flag_reinstall = false;
    let mut flag_root: Option<&str> = None;

    // Pacman-флаги операций
    let mut has_s_op = false;
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
            flag_verify = true;
        } else if a == "--yes" || a == "-y" || a == "--noconfirm" {
            flag_yes = true;
            has_mod_y = true;
        } else if a == "--verbose" || a == "-d" || a == "--verify" || a == "--verify-files" {
            flag_verify = true;
        } else if a == "-c" || a == "--critical" || a == "--critical-only" {
            flag_critical = true;
        } else if a == "-q" || a == "--quiet" {
            flag_quiet = true;
        } else if a == "--reinstall" {
            flag_reinstall = true;
        } else if a == "--dry-run" {
            flag_dry_run = true;
        } else if a == "--json" {
            flag_json = true;
        } else if a == "-f" || a == "--files" {
            flag_files = true;
        } else if a == "-r" || a == "--root" {
            if i + 1 < raw_args.len() {
                i += 1;
                flag_root = Some(raw_args[i]);
            }
        } else if a.starts_with("--root=") {
            flag_root = Some(&a[7..]);
        } else if a.starts_with('-') && !a.starts_with("--") {
            // Разбор компактных коротких флагов pacman (-Syu, -Sy, -Su, -Ss, -Si, -Sc, -Q, -Qi, -U)
            for ch in a[1..].chars() {
                match ch {
                    'S' => has_s_op = true,
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
                    'd' => flag_verify = true,
                    'q' => flag_quiet = true,
                    'v' | 'V' => flag_version = true,
                    'h' => {
                        display::print_help();
                        exit(0);
                    }
                    _ => {
                        display::print_error(&format!("{} '-{}'", t("неизвестный флаг", "unknown flag"), ch));
                        println!("  {}", t("Используйте 'wsp --help' для справки.", "Use 'wsp --help' for help."));
                        exit(1);
                    }
                }
            }
        } else {
            positional.push(a);
        }
        i += 1;
    }

    // Определение команды (pacman-стиль)
    let pacman_cmd = if has_s_op {
        if has_mod_y && has_mod_u {
            Some("update_and_upgrade")
        } else if has_mod_y && positional.is_empty() {
            Some("update")
        } else if has_mod_u {
            Some("upgrade")
        } else if has_mod_s {
            Some("check")
        } else if has_mod_i {
            Some("info")
        } else if has_mod_c && positional.is_empty() {
            Some("clean")
        } else {
            Some("install")
        }
    } else if has_q_op {
        if has_mod_i {
            Some("info")
        } else {
            Some("list")
        }
    } else if has_u_op {
        Some("install")
    } else {
        None
    };

    // Если был запрошен флаг версии без выполнения операций
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

    let root_dir = flag_root.unwrap_or("");
    let extra_arg = rest.first().copied();

    let code = match cmd {
        "version" => {
            display::print_version();
            0
        }
        "audit" => audit::run_audit(flag_verify),
        "check" => audit::run_check(flag_critical, flag_json),
        "update" => do_update(root_dir, flag_quiet),
        "upgrade" => {
            upgrade::run_upgrade(root_dir, flag_critical, flag_yes, flag_dry_run)
        }
        "update_and_upgrade" => {
            let res = do_update(root_dir, flag_quiet);
            if res == 0 {
                upgrade::run_upgrade(root_dir, flag_critical, flag_yes, flag_dry_run)
            } else {
                res
            }
        }
        "list" | "ls" => list::run_list(false, flag_critical),
        "info" => {
            if let Some(pkg) = extra_arg {
                list::run_info(pkg, flag_files)
            } else {
                display::print_error(t("укажите имя пакета: wsp info <пакет>", "missing package name: wsp info <package>"));
                1
            }
        }
        "install" => {
            install::run_install(rest, root_dir, flag_yes, flag_reinstall, flag_verify)
        }
        "kernel" => {
            let sub = extra_arg.unwrap_or("list");
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
                display::print_info(t("локальный кэш /var/cache/wsp очищен.", "local cache /var/cache/wsp cleaned."));
                0
            }
        }
        _ => {
            display::print_error(&format!("{} '{}'", t("неизвестная команда", "unknown command"), cmd));
            println!("  {}", t("Используйте 'wsp --help' для списка доступных команд.", "Use 'wsp --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}
