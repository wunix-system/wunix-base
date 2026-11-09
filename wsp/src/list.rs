//! Модуль просмотра информации о системных пакетах WSP.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};

use crate::db::load_installed_packages;
use crate::display::{print_error, COLOR_GREEN, COLOR_RED, COLOR_RESET};

/// Выводит список установленных пакетов ядра и базы
pub fn run_list(_upgradable_only: bool, critical_only: bool) -> i32 {
    let packages = load_installed_packages();

    println!("Реестр компонентов базовой системы и ядра Wunix:");

    let mut count = 0;
    for pkg in &packages {
        if critical_only && pkg.tier != "critical" {
            continue;
        }

        let tier_label = if pkg.tier == "critical" {
            format!("{}[критический]{}", COLOR_RED, COLOR_RESET)
        } else {
            format!("{}[базовый]{}", COLOR_GREEN, COLOR_RESET)
        };

        println!("  {}{} v{} {}", pkg.name, COLOR_RESET, pkg.version, tier_label);
        if !pkg.description.is_empty() {
            println!("    {}", pkg.description);
        }
        println!("    {}: {}", t("Компонентов на диске", "Files"), pkg.files.len());
        println!();
        count += 1;
    }

    println!("{}: {}", t("Всего системных компонентов", "Total system components"), count);
    0
}

/// Выводит детальную информацию о конкретном системном пакете
pub fn run_info(pkg_name: &str, show_files: bool) -> i32 {
    let packages = load_installed_packages();
    let pkg = match packages.iter().find(|p| p.name == pkg_name) {
        Some(p) => p,
        None => {
            print_error(&format!("{} '{}'", t("системный компонент не найден в базе", "system component not found"), pkg_name));
            return 1;
        }
    };

    println!("Системный компонент: {}", pkg.name);
    println!("  {:<22} {}", t("Имя компонента:", "Package name:"), pkg.name);
    println!("  {:<22} {}", t("Версия ядра/базы:", "Version:"), pkg.version);
    println!("  {:<22} {}", t("Уровень важности:", "System tier:"), pkg.tier);
    println!("  {:<22} {}", t("Описание:", "Description:"), pkg.description);
    println!("  {:<22} {}", t("Файлов в системе:", "Total files:"), pkg.files.len());

    if show_files && !pkg.files.is_empty() {
        println!();
        println!("  {}:", t("Файлы компонента", "Files in system"));
        for f in &pkg.files {
            println!("    {}", f);
        }
    }

    0
}
