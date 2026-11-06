//! Модуль просмотра установленных программ WPM.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};

use crate::display::{print_error, print_field, COLOR_RESET};
use crate::install::load_installed_apps;
use crate::search::get_sample_catalog;

pub fn run_list() -> i32 {
    let apps = load_installed_apps();

    println!("{:<20} {:<10} {}", 
        t("ПРОГРАММА", "APPLICATION"),
        t("ВЕРСИЯ", "VERSION"),
        t("КАТЕГОРИЯ", "CATEGORY")
    );

    if apps.is_empty() {
        println!("{}", t("(нет установленных пользовательских программ)", "(no user applications installed)"));
    } else {
        for a in &apps {
            println!("{:<20} {:<10} {}", a.name, a.version, a.category);
        }
    }

    println!();
    println!("{} {}: {}", t("Всего программ", "Total programs"), apps.len(), COLOR_RESET);
    0
}

pub fn run_info(name: &str) -> i32 {
    let catalog = get_sample_catalog();
    let pkg = catalog.iter().find(|p| p.name == name);

    match pkg {
        Some(p) => {
            println!("{}: {}", t("Пользовательская программа", "User application"), p.name);
            print_field(t("Имя:", "Name:"), &p.name);
            print_field(t("Версия:", "Version:"), &p.version);
            print_field(t("Категория:", "Category:"), &p.category);
            print_field(t("Описание:", "Description:"), &p.description);
            print_field(t("Размер:", "Size:"), &p.size);
            0
        }
        None => {
            print_error(&format!("{} '{}'", t("программа не найдена в репозиториях WPM", "application not found in WPM repositories"), name));
            1
        }
    }
}
