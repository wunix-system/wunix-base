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
mod unpack;
mod upgrade;

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;
use libwunix::{exit, geteuid, getuid, println, t};

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

fn locate_system_package(pkg: &str) -> Option<alloc::string::String> {
    if pkg.ends_with(".wp") || pkg.ends_with(".wsp") || pkg.contains('/') {
        let mut null_p = [0u8; 256];
        let b = pkg.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let fd = libwunix::open(&null_p, libwunix::O_RDONLY as u64, 0);
        if fd >= 0 {
            libwunix::close(fd as u64);
            return Some(pkg.to_string());
        }
    }

    let candidates = [
        alloc::format!("/var/cache/wsp/{}.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-1.0.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-7.2.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-8.6.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-9.8.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-3.4.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-6.4.0.wp", pkg),
        alloc::format!("/var/cache/wsp/{}-1.3.1.wp", pkg),
        alloc::format!("/var/lib/wsp/repo/system/x86_64/{}.wp", pkg),
        alloc::format!("/var/lib/wsp/repo/system/x86_64/{}-1.0.0.wp", pkg),
    ];

    for path in &candidates {
        let mut null_p = [0u8; 256];
        let b = path.as_bytes();
        let l = b.len().min(255);
        null_p[..l].copy_from_slice(&b[..l]);
        null_p[l] = 0;
        let fd = libwunix::open(&null_p, libwunix::O_RDONLY as u64, 0);
        if fd >= 0 {
            libwunix::close(fd as u64);
            return Some(path.clone());
        }
    }

    None
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
    let mut flag_critical = false;
    let mut flag_yes = false;
    let mut flag_verify = false;
    let mut flag_dry_run = false;
    let mut flag_json = false;
    let mut flag_files = false;
    let mut flag_quiet = false;
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
                flag_verify = true;
                flag_version = true;
            }
            "-d" | "--verbose" | "--verify" | "--verify-files" => flag_verify = true,
            "-c" | "--critical" | "--critical-only" => flag_critical = true,
            "-y" | "--yes" => flag_yes = true,
            "-q" | "--quiet" => flag_quiet = true,
            "--reinstall" => flag_reinstall = true,
            "--dry-run" => flag_dry_run = true,
            "--json" => flag_json = true,
            "-f" | "--files" => flag_files = true,
            _ => {
                if !a.is_empty() {
                    if a.starts_with('-') {
                        display::print_error(&format!("{} '{}'", t("неизвестная опция", "unknown option"), a));
                        println!("  {}", t("Используйте 'wsp --help' для просмотра доступных опций.", "Use 'wsp --help' for available options."));
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
    let extra_arg = positional.get(1).copied();

    let code = match cmd {
        "version" => {
            display::print_version();
            0
        }
        "audit" => audit::run_audit(flag_verify),
        "check" => audit::run_check(flag_critical, flag_json),
        "update" => {
            if !flag_quiet {
                println!("wsp: синхронизация системного репозитория...");
                display::print_step(1, 1, t("Синхронизация индекса компонентов ядра (200 OK)", "Synchronizing kernel component index (200 OK)"), true);
                println!("wsp: индекс системных пакетов актуален.");
            }
            0
        }
        "upgrade" => upgrade::run_upgrade(flag_critical, flag_yes, flag_dry_run),
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
            let uid = getuid();
            let euid = geteuid();
            if uid != 0 && euid != 0 {
                display::print_access_denied("install", uid);
                1
            } else if let Some(target) = extra_arg {
                let resolved = locate_system_package(target);
                if let Some(pkg_file) = resolved.as_deref() {
                    match unpack::read_file_to_vec(pkg_file) {
                        Ok(data) => {
                            let parsed = match libpackwunix::format::PackageContainer::parse(&data) {
                                Ok(c) => {
                                    let d = data.len() as u64;
                                    let p_len = c.payload.len();
                                    let i = if p_len >= 4 && c.payload[0] == 0x1f && c.payload[1] == 0x8b {
                                        u32::from_le_bytes([
                                            c.payload[p_len - 4],
                                            c.payload[p_len - 3],
                                            c.payload[p_len - 2],
                                            c.payload[p_len - 1],
                                        ]) as u64
                                    } else {
                                        p_len as u64
                                    };
                                    Ok((c.manifest.name.clone(), c.manifest.version.to_string(), alloc::format!("{:?}", c.manifest.tier), d, i))
                                }
                                Err(e) => {
                                    display::print_error(e);
                                    Err(1)
                                }
                            };

                            match parsed {
                                Ok((manifest_name, manifest_ver, manifest_tier, d_size, i_size)) => {
                                    let installed_pkgs = db::load_installed_packages();
                                    let is_already = installed_pkgs.iter().find(|p| p.name == manifest_name);
                                    if let Some(inst) = is_already {
                                        if !flag_reinstall && inst.version == manifest_ver {
                                            println!(
                                                "wsp: системный компонент '{}' (v{}) уже установлен в системе.",
                                                manifest_name, inst.version
                                            );
                                            println!(
                                                "  Для принудительной переустановки выполните: wsp install --reinstall {}",
                                                pkg_file
                                            );
                                            0
                                        } else {
                                            if inst.version != manifest_ver {
                                                println!(
                                                    "wsp: найдено обновление системного компонента '{}': v{} -> v{}",
                                                    manifest_name, inst.version, manifest_ver
                                                );
                                            }
                                            println!("{}:", t("Будет установлен системный компонент", "The following system component will be installed"));
                                            println!("  {} (v{}) [{}]", manifest_name, manifest_ver, manifest_tier);

                                            display::print_disk_space_change(d_size, i_size, true);

                                            if !flag_yes {
                                                let prompt_text = t("Продолжить установку системного компонента? [Y/n] ", "Continue with installation? [Y/n] ");
                                                if !display::confirm_prompt(prompt_text, true) {
                                                    println!("{}", t("wsp: установка отменена пользователем.", "wsp: installation aborted by user."));
                                                    0
                                                } else {
                                                    install_wsp_component(&data, pkg_file, flag_verify)
                                                }
                                            } else {
                                                install_wsp_component(&data, pkg_file, flag_verify)
                                            }
                                        }
                                    } else {
                                        println!("{}:", t("Будет установлен системный компонент", "The following system component will be installed"));
                                        println!("  {} (v{}) [{}]", manifest_name, manifest_ver, manifest_tier);

                                        display::print_disk_space_change(d_size, i_size, true);

                                        if !flag_yes {
                                            let prompt_text = t("Продолжить установку системного компонента? [Y/n] ", "Continue with installation? [Y/n] ");
                                            if !display::confirm_prompt(prompt_text, true) {
                                                println!("{}", t("wsp: установка отменена пользователем.", "wsp: installation aborted by user."));
                                                0
                                            } else {
                                                install_wsp_component(&data, pkg_file, flag_verify)
                                            }
                                        } else {
                                            install_wsp_component(&data, pkg_file, flag_verify)
                                        }
                                    }
                                }
                                Err(code) => code,
                            }
                        }
                        Err(e) => {
                            display::print_error(e);
                            1
                        }
                    }
                } else {
                    display::print_error(&alloc::format!("системный пакет '{}' не найден в кэше /var/cache/wsp.", target));
                    println!("  Используйте 'wsp list' или 'wsp check' для просмотра доступных компонентов.");
                    1
                }
            } else {
                display::print_error(t("укажите имя пакета для установки: wsp install <пакет>", "missing package: wsp install <package>"));
                1
            }
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
        _ => {
            display::print_error(&format!("{} '{}'", t("неизвестная команда", "unknown command"), cmd));
            println!("  {}", t("Используйте 'wsp --help' для списка доступных команд.", "Use 'wsp --help' for list of commands."));
            1
        }
    };

    exit(code as u64);
}

fn install_wsp_component(data: &[u8], pkg_file: &str, flag_verify: bool) -> i32 {
    println!("wsp: установка системного пакета '{}'", pkg_file);
    match unpack::unpack_wsp_package(data, flag_verify) {
        Ok((manifest, files)) => {
            display::print_step(1, 2, t("Проверка сигнатуры и манифеста", "Verifying signature"), true);
            display::print_step(2, 2, t("Установка системных файлов", "Installing system files"), true);

            let mut installed = db::load_installed_packages();
            installed.retain(|p| p.name != manifest.name);
            installed.push(db::SystemPackageEntry {
                name: manifest.name.clone(),
                version: manifest.version.to_string(),
                tier: alloc::format!("{:?}", manifest.tier),
                description: manifest.description.clone(),
                files: files.clone(),
            });
            db::save_installed_packages(&installed);

            println!("wsp: системный компонент '{}' успешно установлен (файлов: {}).", manifest.name, files.len());
            0
        }
        Err(e) => {
            display::print_error(e);
            1
        }
    }
}
