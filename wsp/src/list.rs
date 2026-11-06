//! Модуль просмотра информации о системных пакетах WSP.
//!
//! Реализует команды:
//! - `wsp list`: перечень установленных базовых пакетов.
//! - `wsp info <пакет>`: детальная карточка системного пакета.

extern crate alloc;
use libwunix::{println, t};

use crate::db::load_installed_packages;
use crate::display::{
    print_card_footer, print_card_header, print_card_row, print_card_separator,
    COLOR_BOLD, COLOR_CYAN, COLOR_GRAY, COLOR_RED, COLOR_RESET,
};

/// Выводит список установленных пакетов системы
pub fn run_list(_upgradable_only: bool, critical_only: bool) -> i32 {
    let packages = load_installed_packages();

    println!("{}{}Установленные системные пакеты (Wunix Base):{}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("┌──────────────────────┬─────────────┬──────────────┬───────────────────────────────┐");
    println!("│ Имя пакета           │ Версия      │ Уровень      │ Описание                      │");
    println!("├──────────────────────┼─────────────┼──────────────┼───────────────────────────────┤");

    let mut count = 0;
    for pkg in &packages {
        if critical_only && pkg.tier != "critical" {
            continue;
        }

        let tier_label = if pkg.tier == "critical" {
            alloc::format!("{}[КРИТИЧЕСКИЙ]{}", COLOR_RED, COLOR_RESET)
        } else {
            alloc::format!("{}[БАЗОВЫЙ]    {}", COLOR_CYAN, COLOR_RESET)
        };

        // Обрезаем описание, если длинное
        let desc = if pkg.description.chars().count() > 30 {
            let mut s: alloc::string::String = pkg.description.chars().take(27).collect();
            s.push_str("...");
            s
        } else {
            pkg.description.clone()
        };

        println!("│ {:<20} │ {:<11} │ {} │ {:<29} │", pkg.name, pkg.version, tier_label, desc);
        count += 1;
    }

    println!("└──────────────────────┴─────────────┴──────────────┴───────────────────────────────┘");
    println!("{}Всего компонентов: {}{}", COLOR_GRAY, count, COLOR_RESET);

    0
}

/// Выводит детальную карточку конкретного системного пакета
pub fn run_info(pkg_name: &str, show_files: bool) -> i32 {
    let packages = load_installed_packages();
    let pkg = match packages.iter().find(|p| p.name == pkg_name) {
        Some(p) => p,
        None => {
            println!("{}[!] Системный пакет '{}' не найден в базе данных WSP.{}", COLOR_RED, pkg_name, COLOR_RESET);
            return 1;
        }
    };

    print_card_header(&alloc::format!("Инфо о пакете: {}", pkg.name));
    print_card_row(t("Имя пакета:", "Package name:"), &pkg.name);
    print_card_row(t("Версия:", "Version:"), &pkg.version);
    print_card_row(t("Уровень важности:", "System tier:"), &pkg.tier);
    print_card_row(t("Описание:", "Description:"), &pkg.description);
    print_card_row(t("Количество файлов:", "Total files:"), &alloc::format!("{}", pkg.files.len()));

    if show_files {
        print_card_separator();
        println!("{}│ Список файлов на диске:{}", COLOR_GRAY, COLOR_RESET);
        for f in &pkg.files {
            println!("{}│   • {}{}", COLOR_GRAY, f, COLOR_RESET);
        }
    }

    print_card_footer();
    0
}
