//! Модуль просмотра установленных программ WPM.

extern crate alloc;
use libwunix::{println, t};

use crate::display::{
    print_card_footer, print_card_header, print_card_row, COLOR_BOLD, COLOR_CYAN, COLOR_GRAY,
    COLOR_RED, COLOR_RESET,
};
use crate::install::load_installed_apps;
use crate::search::get_sample_catalog;

pub fn run_list() -> i32 {
    let apps = load_installed_apps();
    println!("{}{}Установленные прикладные программы (WPM):{}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("┌──────────────────────┬─────────────┬───────────────────────────────┐");
    println!("│ Имя программы        │ Версия      │ Категория                     │");
    println!("├──────────────────────┼─────────────┼───────────────────────────────┤");

    if apps.is_empty() {
        println!("│ (Пока нет установленных пользовательских программ)                   │");
    } else {
        for a in &apps {
            println!("│ {:<20} │ {:<11} │ {:<29} │", a.name, a.version, a.category);
        }
    }

    println!("└──────────────────────┴─────────────┴───────────────────────────────┘");
    println!("{}Всего программ: {}{}", COLOR_GRAY, apps.len(), COLOR_RESET);
    0
}

pub fn run_info(name: &str) -> i32 {
    let catalog = get_sample_catalog();
    let pkg = catalog.iter().find(|p| p.name == name);

    match pkg {
        Some(p) => {
            print_card_header(&alloc::format!("Инфо о программе: {}", p.name));
            print_card_row(t("Имя:", "Name:"), &p.name);
            print_card_row(t("Версия:", "Version:"), &p.version);
            print_card_row(t("Категория:", "Category:"), &p.category);
            print_card_row(t("Описание:", "Description:"), &p.description);
            print_card_row(t("Размер:", "Size:"), &p.size);
            print_card_footer();
            0
        }
        None => {
            println!("{}[!] Программа '{}' не найдена в репозиториях WPM.{}", COLOR_RED, name, COLOR_RESET);
            1
        }
    }
}
