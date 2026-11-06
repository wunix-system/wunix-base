//! Модуль поиска прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{println, t};

use crate::display::{COLOR_BOLD, COLOR_GREEN, COLOR_RESET};

pub struct CatalogPackage {
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub size: String,
    pub installed: bool,
}

pub fn get_sample_catalog() -> Vec<CatalogPackage> {
    alloc::vec![
        CatalogPackage {
            name: "tinycc".to_string(),
            version: "0.9.27".to_string(),
            category: "programs-dev".to_string(),
            description: "Быстрый компилятор Си (tcc) с поддержкой Wunix".to_string(),
            size: "244.5 KiB".to_string(),
            installed: false,
        },
        CatalogPackage {
            name: "make".to_string(),
            version: "4.4.1".to_string(),
            category: "programs-dev".to_string(),
            description: "Утилита автоматизации сборки проектов".to_string(),
            size: "410.0 KiB".to_string(),
            installed: false,
        },
        CatalogPackage {
            name: "doom".to_string(),
            version: "1.1.0".to_string(),
            category: "applications".to_string(),
            description: "Классический 3D шутер DOOM для Limine Framebuffer".to_string(),
            size: "4.50 MiB".to_string(),
            installed: false,
        },
        CatalogPackage {
            name: "git".to_string(),
            version: "2.44.0".to_string(),
            category: "programs-dev".to_string(),
            description: "Распределенная система управления версиями".to_string(),
            size: "6.80 MiB".to_string(),
            installed: false,
        },
    ]
}

pub fn run_search(query: &str) -> i32 {
    let catalog = get_sample_catalog();
    println!("{}: \"{}\"\n", t("Поиск пакетов", "Search query"), query);

    let mut found = 0;
    for pkg in &catalog {
        let match_name = pkg.name.contains(query);
        let match_desc = pkg.description.contains(query);
        let match_cat = pkg.category.contains(query);

        if query.is_empty() || match_name || match_desc || match_cat {
            found += 1;
            let inst_status = if pkg.installed {
                format!("{}[{}]", COLOR_GREEN, t("установлен", "installed"))
            } else {
                format!("[{}]", t("доступен", "available"))
            };

            println!("  {}{} ({}){} [{}]", COLOR_BOLD, pkg.name, pkg.version, COLOR_RESET, pkg.category);
            println!("    {}", pkg.description);
            println!("    {}: {} | {}: {}\n", t("Размер", "Size"), pkg.size, t("Статус", "Status"), inst_status);
        }
    }

    if found == 0 {
        println!("{}", t("wpm: совпадений не найдено.", "wpm: no matching packages found."));
    } else {
        println!("{}: {}", t("Найдено пакетов", "Total packages found"), found);
    }

    0
}
