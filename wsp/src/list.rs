//! Модуль просмотра информации о системных пакетах WSP.
//!
//! Реализует команды:
//! - `wsp list`: перечень установленных базовых пакетов.
//! - `wsp info <пакет>`: детальная карточка системного пакета.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};

use crate::db::load_installed_packages;
use crate::display::{print_error, print_field, COLOR_GREEN, COLOR_RED, COLOR_RESET};

/// Выводит список установленных пакетов системы в стандартном табличном формате
pub fn run_list(_upgradable_only: bool, critical_only: bool) -> i32 {
    let packages = load_installed_packages();

    println!("{:<20} {:<10} {:<12} {}", 
        t("ПАКЕТ", "PACKAGE"),
        t("ВЕРСИЯ", "VERSION"),
        t("УРОВЕНЬ", "TIER"),
        t("ОПИСАНИЕ", "DESCRIPTION")
    );

    let mut count = 0;
    for pkg in &packages {
        if critical_only && pkg.tier != "critical" {
            continue;
        }

        let tier_label = if pkg.tier == "critical" {
            format!("{}[критич]{}", COLOR_RED, COLOR_RESET)
        } else {
            format!("{}[базовый]{}", COLOR_GREEN, COLOR_RESET)
        };

        // Обрезаем описание, если длинное
        let desc = if pkg.description.chars().count() > 36 {
            let mut s: alloc::string::String = pkg.description.chars().take(33).collect();
            s.push_str("...");
            s
        } else {
            pkg.description.clone()
        };

        println!("{:<20} {:<10} {:<20} {}", pkg.name, pkg.version, tier_label, desc);
        count += 1;
    }

    println!();
    println!("{} {}: {}", t("Всего системных компонентов", "Total system components"), count, COLOR_RESET);

    0
}

/// Выводит детальную информацию о конкретном системном пакете
pub fn run_info(pkg_name: &str, show_files: bool) -> i32 {
    let packages = load_installed_packages();
    let pkg = match packages.iter().find(|p| p.name == pkg_name) {
        Some(p) => p,
        None => {
            print_error(&format!("{} '{}'", t("системный пакет не найден", "system package not found"), pkg_name));
            return 1;
        }
    };

    println!("{}: {}", t("Системный пакет", "System package"), pkg.name);
    print_field(t("Имя пакета:", "Package name:"), &pkg.name);
    print_field(t("Версия:", "Version:"), &pkg.version);
    print_field(t("Уровень важности:", "System tier:"), &pkg.tier);
    print_field(t("Описание:", "Description:"), &pkg.description);
    print_field(t("Всего файлов:", "Total files:"), &format!("{}", pkg.files.len()));

    if show_files {
        println!();
        println!("{}:", t("  Файлы в системе", "  Installed files"));
        for f in &pkg.files {
            println!("    {}", f);
        }
    }

    0
}
