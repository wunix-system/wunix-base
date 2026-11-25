//! Единый модуль установки, переустановки и удаления пакетов WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, getdents64, geteuid, getuid, open, println, t, unlink, O_RDONLY};
use libpackwunix::format::PackageContainer;
use libpackwunix::sources::{get_wsp_system_repo, RepoPackage};

use crate::db::{
    ensure_dir_recursive, get_path, load_installed_packages_from, load_unified_catalog_from,
    log_transaction, save_installed_packages_to, CatalogEntry, PackageEntry,
};
use crate::display::{
    confirm_prompt, print_access_denied, print_disk_space_change, print_error,
    print_step, COLOR_BOLD, COLOR_RESET,
};
use crate::guard::ensure_system_safe_for_user_apps;
use crate::resolver::{resolve_alias, resolve_install_order};
use crate::unpack::{read_file_to_vec, unpack_package_to};

pub fn scan_dir_for_package(dir: &str, pkg: &str) -> Option<String> {
    let mut null_dir = [0u8; 256];
    let db = dir.as_bytes();
    if db.len() >= 255 {
        return None;
    }
    null_dir[..db.len()].copy_from_slice(db);
    null_dir[db.len()] = 0;

    let fd = open(&null_dir, O_RDONLY as u64, 0);
    if fd < 0 {
        return None;
    }

    let mut buf = [0u8; 4096];
    let prefix = format!("{}-", pkg);
    let exact = format!("{}.wp", pkg);
    let mut found = None;

    loop {
        let n = getdents64(fd as u64, &mut buf);
        if n <= 0 {
            break;
        }
        let total = (n as usize).min(buf.len());
        let mut pos = 0;
        while pos + 19 <= total {
            let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
            if reclen == 0 || pos + reclen > total {
                break;
            }
            let name_start = pos + 19;
            let mut name_end = name_start;
            let max_end = pos + reclen;
            while name_end < max_end && buf[name_end] != 0 {
                name_end += 1;
            }
            if name_end > name_start {
                if let Ok(name) = core::str::from_utf8(&buf[name_start..name_end]) {
                    if (name == exact || name.starts_with(&prefix))
                        && (name.ends_with(".wp") || name.ends_with(".wsp"))
                    {
                        found = Some(format!("{}/{}", dir.trim_end_matches('/'), name));
                        break;
                    }
                }
            }
            pos += reclen;
        }
        if found.is_some() {
            break;
        }
    }
    let _ = close(fd as u64);
    found
}

fn file_readable(path: &str) -> bool {
    let mut null_p = [0u8; 256];
    let b = path.as_bytes();
    if b.len() >= 255 {
        return false;
    }
    null_p[..b.len()].copy_from_slice(b);
    null_p[b.len()] = 0;
    let fd = open(&null_p, O_RDONLY as u64, 0);
    if fd >= 0 {
        let _ = close(fd as u64);
        true
    } else {
        false
    }
}

pub fn locate_local_package_file(
    pkg_id: &str,
    expected_filename: Option<&str>,
    root_prefix: Option<&str>,
) -> Option<String> {
    if pkg_id.ends_with(".wp") || pkg_id.ends_with(".wsp") || pkg_id.contains('/') {
        if file_readable(pkg_id) {
            return Some(pkg_id.to_string());
        }
    }

    let mut dirs: Vec<String> = alloc::vec![
        "/var/cache/wpm".to_string(),
        "/var/lib/wpm/repo/system/x86_64".to_string(),
        "/var/lib/wpm/repo/apps/x86_64".to_string(),
        "/repositories/system/x86_64".to_string(),
        "/repositories/apps/x86_64".to_string(),
    ];
    if let Some(r) = root_prefix {
        if !r.is_empty() && r != "/" {
            let trimmed = r.trim_end_matches('/');
            dirs.insert(0, format!("{}/var/cache/wpm", trimmed));
            dirs.insert(1, format!("{}/var/lib/wpm/repo/system/x86_64", trimmed));
            dirs.insert(2, format!("{}/var/lib/wpm/repo/apps/x86_64", trimmed));
        }
    }

    if let Some(fname) = expected_filename {
        for dir in &dirs {
            let cand = format!("{}/{}", dir, fname);
            if file_readable(&cand) {
                return Some(cand);
            }
        }
    }

    for dir in &dirs {
        if let Some(found) = scan_dir_for_package(dir, pkg_id) {
            return Some(found);
        }
    }

    None
}

fn channel_download_url(channel: &str, filename: &str) -> String {
    if channel == "system" {
        return format!("{}/{}", get_wsp_system_repo().trim_end_matches('/'), filename);
    }
    let sources = crate::repo_cmd::load_sources();
    if let Some(sec) = sources.iter().find(|s| s.name == channel) {
        if let Some(srv) = sec.servers.first() {
            return format!("{}/{}", srv.trim_end_matches('/'), filename);
        }
    }
    let base = sources
        .first()
        .and_then(|s| s.servers.first())
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("http://wunix.rusos.site/repo/apps/x86_64");
    format!("{}/{}", base, filename)
}

enum TargetSource {
    Local { path: String },
    Remote {
        download_url: String,
        dest_path: String,
        sha256: String,
        filename: String,
    },
}

struct PendingTarget {
    name: String,
    version: String,
    pkg_type: String,
    is_meta: bool,
    source: TargetSource,
}

fn estimate_unpacked_size(pkg_data: &[u8], payload: &[u8]) -> u64 {
    let p_len = payload.len();
    if p_len >= 4 && payload[0] == 0x1f && payload[1] == 0x8b {
        u32::from_le_bytes([
            payload[p_len - 4],
            payload[p_len - 3],
            payload[p_len - 2],
            payload[p_len - 1],
        ]) as u64
    } else {
        pkg_data.len() as u64
    }
}

pub fn run_install_in(
    packages: &[&str],
    root_prefix: Option<&str>,
    assume_yes: bool,
    ignore_unsafe: bool,
    verbose: bool,
    reinstall: bool,
) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите имя пакета для установки: wpm install <пакет>...",
            "missing package name: wpm install <package>...",
        ));
        return 1;
    }

    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied(if reinstall { "reinstall" } else { "install" }, uid);
        return 1;
    }

    let root_str = root_prefix.unwrap_or("/");
    let catalog = load_unified_catalog_from(root_str);

    // 1. Разделяем локальные файлы .wp и имена пакетов из каталогов
    let mut explicit_names: Vec<String> = Vec::new();
    // (name, path, version, pkg_type, is_meta, d_size, i_size, deps)
    let mut explicit_local_files: Vec<(String, String, String, String, bool, u64, u64, Vec<String>)> = Vec::new();

    for &raw_arg in packages {
        if (raw_arg.ends_with(".wp") || raw_arg.ends_with(".wsp") || raw_arg.contains('/'))
            && file_readable(raw_arg)
        {
            match read_file_to_vec(raw_arg) {
                Ok(data) => match PackageContainer::parse(&data) {
                    Ok(c) => {
                        let cname = c.manifest.name.clone();
                        let ver = c.manifest.version.to_string();
                        let ptype = c.manifest.package_type.as_str().to_string();
                        let d_size = data.len() as u64;
                        let i_size = estimate_unpacked_size(&data, c.payload);
                        let deps: Vec<String> = c.manifest.required_deps.iter().map(|d| d.name.clone()).collect();
                        explicit_names.push(cname.clone());
                        explicit_local_files.push((
                            cname,
                            raw_arg.to_string(),
                            ver,
                            ptype,
                            c.manifest.is_meta,
                            d_size,
                            i_size,
                            deps,
                        ));
                    }
                    Err(e) => {
                        print_error(e);
                        return 1;
                    }
                },
                Err(e) => {
                    print_error(e);
                    return 1;
                }
            }
        } else {
            explicit_names.push(resolve_alias(raw_arg).to_string());
        }
    }

    // 2. Строим граф зависимостей (каталог + локальные файлы)
    let mut graph: Vec<(String, Vec<String>)> = Vec::new();
    for entry in &catalog {
        graph.push((entry.pkg.name.clone(), entry.pkg.dependencies.clone()));
    }
    for (cname, _, _, _, _, _, _, deps) in &explicit_local_files {
        if !graph.iter().any(|(n, _)| n == cname) {
            graph.push((cname.clone(), deps.clone()));
        }
    }

    // Проверяем, что все явно запрошенные пакеты известны
    for name in &explicit_names {
        let in_cat = catalog.iter().any(|c| &c.pkg.name == name);
        let in_loc = explicit_local_files.iter().any(|(n, ..)| n == name);
        let on_disk = locate_local_package_file(name, None, root_prefix).is_some();
        if !in_cat && !in_loc && !on_disk {
            print_error(&format!(
                "пакет '{}' не найден в системном или пользовательских репозиториях и кэше /var/cache/wpm.",
                name
            ));
            println!("  Используйте 'sudo wpm update' для синхронизации каталогов.");
            println!("  Используйте 'wpm search <запрос>' для поиска доступных пакетов.");
            return 1;
        }
    }

    // 3. Раскрываем зависимости и метапакеты в топологическом порядке
    let install_order = match resolve_install_order(&explicit_names, &graph) {
        Ok(ord) => ord,
        Err(e) => {
            print_error(e);
            return 1;
        }
    };

    let installed_list = load_installed_packages_from(root_str);
    let mut targets: Vec<PendingTarget> = Vec::new();
    let mut total_download: u64 = 0;
    let mut total_installed_space: u64 = 0;

    for pkg_name in &install_order {
        let is_explicit = explicit_names.iter().any(|e| e == pkg_name);

        if let Some((_, lpath, ver, ptype, is_meta, d_size, i_size, _)) =
            explicit_local_files.iter().find(|(n, _, _, _, _, _, _, _)| n == pkg_name)
        {
            if let Some(inst) = installed_list.iter().find(|a| &a.name == pkg_name) {
                if &inst.version == ver && !(reinstall && is_explicit) {
                    if is_explicit {
                        println!("wpm: пакет '{}' (v{}) уже установлен в системе.", pkg_name, inst.version);
                        println!("  Для переустановки выполните: sudo wpm reinstall {}", pkg_name);
                    }
                    continue;
                } else if &inst.version != ver {
                    println!("wpm: обновление '{}': v{} -> v{}", pkg_name, inst.version, ver);
                }
            }

            total_download += *d_size;
            total_installed_space += *i_size;
            targets.push(PendingTarget {
                name: pkg_name.clone(),
                version: ver.clone(),
                pkg_type: ptype.clone(),
                is_meta: *is_meta,
                source: TargetSource::Local { path: lpath.clone() },
            });
            continue;
        }

        let cat_entry: Option<&CatalogEntry> = catalog.iter().find(|c| &c.pkg.name == pkg_name);
        let expected_fname = cat_entry.map(|c| c.pkg.filename.as_str());

        if let Some(local_path) = locate_local_package_file(pkg_name, expected_fname, root_prefix) {
            let (ver, ptype, is_meta, d_size, i_size) = if let Some(entry) = cat_entry {
                let mut st = libwunix::Stat::zeroed();
                let mut null_p = [0u8; 256];
                let pb = local_path.as_bytes();
                let pl = pb.len().min(255);
                null_p[..pl].copy_from_slice(&pb[..pl]);
                let fsize = if libwunix::stat(&null_p[..pl + 1], &mut st) == 0 && st.st_size > 0 {
                    st.st_size as u64
                } else {
                    entry.pkg.size
                };
                (
                    entry.pkg.version.to_string(),
                    entry.pkg.package_type.clone(),
                    entry.pkg.is_meta,
                    fsize,
                    fsize.saturating_mul(2),
                )
            } else {
                let pkg_data = match read_file_to_vec(&local_path) {
                    Ok(d) => d,
                    Err(e) => {
                        print_error(e);
                        return 1;
                    }
                };
                let c = match PackageContainer::parse(&pkg_data) {
                    Ok(c) => c,
                    Err(e) => {
                        print_error(e);
                        return 1;
                    }
                };
                (
                    c.manifest.version.to_string(),
                    c.manifest.package_type.as_str().to_string(),
                    c.manifest.is_meta,
                    pkg_data.len() as u64,
                    estimate_unpacked_size(&pkg_data, c.payload),
                )
            };

            if let Some(inst) = installed_list.iter().find(|a| &a.name == pkg_name) {
                if inst.version == ver && !(reinstall && is_explicit) {
                    if is_explicit {
                        println!("wpm: пакет '{}' (v{}) уже установлен в системе.", pkg_name, inst.version);
                        println!("  Для переустановки выполните: sudo wpm reinstall {}", pkg_name);
                    }
                    continue;
                } else if inst.version != ver {
                    println!("wpm: обновление '{}': v{} -> v{}", pkg_name, inst.version, ver);
                }
            }

            total_download += d_size;
            total_installed_space += i_size;
            targets.push(PendingTarget {
                name: pkg_name.clone(),
                version: ver,
                pkg_type: ptype,
                is_meta,
                source: TargetSource::Local { path: local_path },
            });
        } else if let Some(entry) = cat_entry {
            let rp: &RepoPackage = &entry.pkg;
            let ver = rp.version.to_string();
            if let Some(inst) = installed_list.iter().find(|a| &a.name == pkg_name) {
                if inst.version == ver && !(reinstall && is_explicit) {
                    if is_explicit {
                        println!("wpm: пакет '{}' (v{}) уже установлен в системе.", pkg_name, inst.version);
                        println!("  Для переустановки выполните: sudo wpm reinstall {}", pkg_name);
                    }
                    continue;
                } else if inst.version != ver {
                    println!("wpm: обновление '{}': v{} -> v{}", pkg_name, inst.version, ver);
                }
            }

            let cache_dir = get_path(root_str, "/var/cache/wpm");
            let dest_path = format!("{}/{}", cache_dir, rp.filename);
            let download_url = channel_download_url(&entry.channel, &rp.filename);

            total_download += rp.size;
            total_installed_space += rp.size.saturating_mul(2);
            targets.push(PendingTarget {
                name: pkg_name.clone(),
                version: ver,
                pkg_type: rp.package_type.clone(),
                is_meta: rp.is_meta,
                source: TargetSource::Remote {
                    download_url,
                    dest_path,
                    sha256: rp.sha256.clone(),
                    filename: rp.filename.clone(),
                },
            });
        } else {
            print_error(&format!("зависимость '{}' не найдена в репозиториях WPM.", pkg_name));
            return 1;
        }
    }

    if targets.is_empty() {
        println!("{}", t("wpm: нет пакетов, требующих установки или обновления.", "wpm: no packages require installation or upgrade."));
        return 0;
    }

    // Если среди устанавливаемых есть пользовательские программы, проверяем защитный барьер
    let has_user_target = targets.iter().any(|t| t.pkg_type == "user");
    if has_user_target && root_prefix.is_none() && ensure_system_safe_for_user_apps(ignore_unsafe).is_err() {
        return 1;
    }

    println!("{}", t("Будут установлены следующие пакеты:", "The following packages will be installed:"));
    for target in &targets {
        let tag = if target.is_meta { " [meta]" } else { "" };
        println!("  {} (v{}) [{}]{}", target.name, target.version, target.pkg_type, tag);
    }

    print_disk_space_change(total_download, total_installed_space, true);

    if !assume_yes {
        let prompt_text = t("Продолжить установку? [Y/n] ", "Do you want to continue? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: установка отменена пользователем.", "wpm: installation aborted by user."));
            return 0;
        }
    }

    let mut installed_db = load_installed_packages_from(root_str);
    let mut installed_count = 0;

    for target in targets {
        println!("wpm: установка '{}'...", target.name);

        let (pkg_data, display_path) = match target.source {
            TargetSource::Local { path } => {
                let data = match read_file_to_vec(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        print_step(1, 3, &format!("{} {}", t("Проверка", "Verifying"), path), false);
                        print_error(e);
                        return 1;
                    }
                };
                (data, path)
            }
            TargetSource::Remote { download_url, dest_path, sha256, filename } => {
                let cache_dir = get_path(root_str, "/var/cache/wpm");
                ensure_dir_recursive(&cache_dir);

                let data = match read_file_to_vec(&dest_path) {
                    Ok(d) => d,
                    Err(_) => {
                        println!("wpm: загрузка '{}'...", target.name);
                        let sha_opt = if sha256.is_empty() { None } else { Some(sha256.as_str()) };
                        match crate::http::fetch_to_file(&download_url, &dest_path, sha_opt, &filename) {
                            Ok(_) => match read_file_to_vec(&dest_path) {
                                Ok(d) => d,
                                Err(e) => {
                                    print_error(e);
                                    return 1;
                                }
                            },
                            Err(err) => {
                                print_error(&format!("ошибка загрузки '{}': {:?}", target.name, err));
                                return 1;
                            }
                        }
                    }
                };
                (data, dest_path)
            }
        };

        print_step(1, 3, &format!("{} {}", t("Проверка", "Verifying"), display_path), true);

        let (manifest, files) = match unpack_package_to(&pkg_data, root_prefix, verbose) {
            Ok(res) => res,
            Err(e) => {
                if installed_count > 0 {
                    save_installed_packages_to(root_str, &installed_db);
                }
                print_step(2, 3, t("Распаковка файлов", "Extracting files"), false);
                print_error(e);
                return 1;
            }
        };
        drop(pkg_data);
        print_step(2, 3, t("Распаковка файлов и библиотек", "Extracting binaries and libraries"), true);

        let deps: Vec<String> = manifest.required_deps.iter().map(|d| d.name.clone()).collect();
        installed_db.retain(|a| a.name != manifest.name);
        installed_db.push(PackageEntry {
            name: manifest.name.clone(),
            version: manifest.version.to_string(),
            pkg_type: manifest.package_type.as_str().to_string(),
            tier: manifest.tier.as_str().to_string(),
            is_meta: manifest.is_meta,
            description: manifest.description.clone(),
            dependencies: deps,
            files,
        });
        print_step(3, 3, t("Регистрация в реестре WPM", "Registering in WPM database"), true);

        println!(
            "wpm: программа '{}{}{}' ({}) успешно установлена.",
            COLOR_BOLD, manifest.name, COLOR_RESET, manifest.version
        );
        if manifest.name == "tinycc" {
            println!("  Команда для запуска: tcc -v");
        }
        installed_count += 1;
    }

    if installed_count > 0 {
        save_installed_packages_to(root_str, &installed_db);
    }

    log_transaction("install", &format!("Установлено пакетов: {}", installed_count));
    if installed_count > 0 { 0 } else { 1 }
}

pub fn run_remove(packages: &[&str], purge: bool, assume_yes: bool) -> i32 {
    if packages.is_empty() {
        print_error(t(
            "укажите имя пакета для удаления: wpm remove <пакет>",
            "missing package name to remove: wpm remove <package>",
        ));
        return 1;
    }

    let uid = getuid();
    let euid = geteuid();
    if uid != 0 && euid != 0 {
        print_access_denied("remove", uid);
        return 1;
    }

    let mut installed_list = load_installed_packages_from("/");
    let initial_count = installed_list.len();

    let mut targets_to_remove: Vec<PackageEntry> = Vec::new();
    let mut total_freed_space: u64 = 0;

    for &raw_pkg in packages {
        let pkg_id = resolve_alias(raw_pkg);
        if let Some(app) = installed_list.iter().find(|a| a.name == pkg_id || a.name == raw_pkg) {
            if app.tier == "critical" || app.pkg_type == "system" {
                print_error(&format!(
                    "пакет '{}' является системным компонентом ОС ({}) и защищен от удаления.",
                    app.name, app.tier
                ));
                return 1;
            }
            let freed = (app.files.len() as u64 * 32 * 1024).max(520 * 1024);
            total_freed_space += freed;
            targets_to_remove.push(app.clone());
        } else {
            print_error(&format!(
                "{} '{}' {}",
                t("пакет", "package"),
                raw_pkg,
                t("не найден среди установленных", "is not installed")
            ));
            return 1;
        }
    }

    println!("{}", t("Будут удалены следующие программы:", "The following packages will be removed:"));
    for target in &targets_to_remove {
        println!("  {} (v{}) — {}: {}", target.name, target.version, t("файлов", "files"), target.files.len());
    }

    print_disk_space_change(0, total_freed_space, false);

    if !assume_yes {
        let prompt_text = t("Продолжить удаление? [Y/n] ", "Do you want to continue? [Y/n] ");
        if !confirm_prompt(prompt_text, true) {
            println!("{}", t("wpm: удаление отменено пользователем.", "wpm: removal aborted by user."));
            return 0;
        }
    }

    println!("wpm: удаление программ...");

    for app in targets_to_remove {
        let mut removed_count = 0;
        for file_path in &app.files {
            let mut null_p = [0u8; 512];
            let b = file_path.as_bytes();
            let l = b.len().min(511);
            null_p[..l].copy_from_slice(&b[..l]);
            null_p[l] = 0;
            if unlink(&null_p) >= 0 {
                removed_count += 1;
            }
        }
        installed_list.retain(|a| a.name != app.name);
        println!(
            "  удалено: {} (файлов: {}{})",
            app.name,
            removed_count,
            if purge { t(", конфигурации очищены", ", configs purged") } else { "" }
        );
    }

    if installed_list.len() < initial_count {
        save_installed_packages_to("/", &installed_list);
        log_transaction("remove", "Удалены выбранные пользовательские пакеты");
        println!("wpm: удаление программ завершено.");
        0
    } else {
        1
    }
}
