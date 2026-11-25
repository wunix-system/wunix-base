//! Модуль просмотра установленных пакетов и информации о них в WPM.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};
use libpackwunix::version::Version;

use crate::db::{load_installed_packages_from, load_unified_catalog_from};
use crate::display::{print_error, COLOR_GREEN, COLOR_RED, COLOR_RESET, COLOR_YELLOW};
use crate::progress::format_bytes;
use crate::resolver::resolve_alias;
use crate::search::get_aliases_for;

pub fn run_list_in(root: &str, filter_critical: bool) -> i32 {
    let pkgs = load_installed_packages_from(root);

    println!("{}:", t("Установленные пакеты (WPM)", "Installed packages (WPM)"));
    println!();

    let mut shown = 0;
    for p in &pkgs {
        if filter_critical && p.tier != "critical" && p.name != "wunix-kernel" {
            continue;
        }
        shown += 1;
        let tier_badge = if p.tier == "critical" {
            format!("{}[critical]{}", COLOR_RED, COLOR_RESET)
        } else if p.is_meta {
            format!("{}[meta]{}", COLOR_YELLOW, COLOR_RESET)
        } else {
            format!("{}[{}]{}", COLOR_GREEN, p.pkg_type, COLOR_RESET)
        };
        println!("  {:<18} v{:<14} {}", p.name, p.version, tier_badge);
        if !p.description.is_empty() {
            println!("    {}", p.description);
        }
    }

    if shown == 0 {
        println!("  {}", t("(нет установленных пакетов)", "(no installed packages)"));
    }

    println!();
    println!("{}: {}", t("Всего пакетов", "Total packages"), shown);
    0
}

#[allow(dead_code)]
pub fn run_check(root_prefix: Option<&str>) -> i32 {
    crate::audit::run_check(root_prefix, false, false)
}

pub fn run_info(name: &str, show_files: bool) -> i32 {
    let installed = load_installed_packages_from("/");
    let catalog = load_unified_catalog_from("/");

    let pkg_id = resolve_alias(name);
    let inst_match = installed.iter().find(|a| a.name == pkg_id || a.name == name);
    let cat_match = catalog.iter().find(|c| {
        c.pkg.name == pkg_id
            || c.pkg.name == name
            || get_aliases_for(&c.pkg.name).iter().any(|al| al == name)
    });

    if inst_match.is_none() && cat_match.is_none() {
        print_error(&format!("пакет '{}' не найден в базе и каталогах WPM", name));
        return 1;
    }

    println!("Информация о пакете '{}':", name);

    if let Some(a) = inst_match {
        println!("  Имя:         {}", a.name);
        println!("  Версия:      {}", a.version);
        println!("  Тип / Tier:  {} / {}{}", a.pkg_type, a.tier, if a.is_meta { " (метапакет)" } else { "" });
        println!("  Статус:      {}[установлен в системе]{}", COLOR_GREEN, COLOR_RESET);
        if !a.description.is_empty() {
            println!("  Описание:    {}", a.description);
        }
        if !a.dependencies.is_empty() {
            println!("  Зависимости: {}", a.dependencies.join(", "));
        }
        if show_files && !a.files.is_empty() {
            println!("  Файлы ({}):", a.files.len());
            for f in &a.files {
                println!("    - {}", f);
            }
        }
        if let Some(c) = cat_match {
            let inst_ver = Version::parse(&a.version).unwrap_or(Version::new(0, 1, 0));
            if c.pkg.version.is_newer_than(&inst_ver) {
                println!("  Доступно:    v{} (выполните: sudo wpm upgrade)", c.pkg.version);
            }
        }
    } else if let Some(c) = cat_match {
        let p = &c.pkg;
        println!("  Имя:         {}", p.name);
        println!("  Версия:      {}", p.version);
        println!("  Канал:       {}", c.channel);
        println!("  Тип / Tier:  {} / {}{}", p.package_type, p.tier, if p.is_meta { " (метапакет)" } else { "" });
        println!("  Описание:    {}", p.description);
        if !p.dependencies.is_empty() {
            println!("  Зависимости: {}", p.dependencies.join(", "));
        }
        println!("  Размер:      {}", format_bytes(p.size));
        println!("  Статус:      [доступен в репозитории]");
        println!();
        println!("  Установка: sudo wpm install {}", name);
    }

    0
}
