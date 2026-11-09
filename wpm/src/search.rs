//! Модуль поиска прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{println, t};

use crate::display::{COLOR_BOLD, COLOR_CYAN, COLOR_DIM, COLOR_GREEN, COLOR_RESET};
use crate::install::load_installed_apps;

pub struct CatalogPackage {
    pub name: String,
    pub aliases: Vec<String>,
    pub version: String,
    pub category: String,
    pub description: String,
    pub size: String,
}

pub fn get_wpm_catalog() -> Vec<CatalogPackage> {
    alloc::vec![
        CatalogPackage {
            name: "tinycc".to_string(),
            aliases: alloc::vec!["tcc".to_string(), "cc".to_string()],
            version: "0.9.27".to_string(),
            category: "programs-dev".to_string(),
            description: "Быстрый и легковесный компилятор Си (tcc) для Wunix".to_string(),
            size: "244.5 KiB".to_string(),
        },
        CatalogPackage {
            name: "make".to_string(),
            aliases: alloc::vec![],
            version: "4.4.1".to_string(),
            category: "programs-dev".to_string(),
            description: "Классическая утилита автоматизации сборки проектов".to_string(),
            size: "410.0 KiB".to_string(),
        },
        CatalogPackage {
            name: "git".to_string(),
            aliases: alloc::vec![],
            version: "2.44.0".to_string(),
            category: "programs-dev".to_string(),
            description: "Распределенная система управления версиями исходного кода".to_string(),
            size: "6.80 MiB".to_string(),
        },
        CatalogPackage {
            name: "doom".to_string(),
            aliases: alloc::vec![],
            version: "1.1.0".to_string(),
            category: "applications".to_string(),
            description: "Легендарный шутер DOOM для графического буфера кадра Wunix".to_string(),
            size: "4.50 MiB".to_string(),
        },
    ]
}

pub fn run_search(query: &str) -> i32 {
    let catalog = get_wpm_catalog();
    let installed = load_installed_apps();

    println!();
    println!("  {}{}◆ WPM ── Каталог прикладных программ (поиск: \"{}\"){}", COLOR_BOLD, COLOR_GREEN, query, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);

    let mut found = 0;
    for pkg in &catalog {
        let match_name = pkg.name.contains(query);
        let match_alias = pkg.aliases.iter().any(|a| a.contains(query));
        let match_desc = pkg.description.contains(query);
        let match_cat = pkg.category.contains(query);

        if query.is_empty() || match_name || match_alias || match_desc || match_cat {
            found += 1;
            let is_installed = installed.iter().any(|a| a.name == pkg.name || pkg.aliases.contains(&a.name));

            let status_badge = if is_installed {
                format!("{}[установлен]{}", COLOR_GREEN, COLOR_RESET)
            } else {
                format!("{}[доступен для установки]{}", COLOR_CYAN, COLOR_RESET)
            };

            println!("  {}● {}{} • v{} [{}]", COLOR_BOLD, pkg.name, COLOR_RESET, pkg.version, pkg.category);
            println!("    {}", pkg.description);

            if !pkg.aliases.is_empty() {
                let alias_list = pkg.aliases.join(", ");
                println!("    {}: {}", t("Алиасы команд", "Command aliases"), alias_list);
            }

            println!(
                "    {}: {} | {}: {}",
                t("Размер", "Size"),
                pkg.size,
                t("Статус", "Status"),
                status_badge
            );

            if !is_installed {
                println!(
                    "    {}: {}sudo wpm install {}{}",
                    t("Установка", "Install command"),
                    COLOR_BOLD,
                    if !pkg.aliases.is_empty() { &pkg.aliases[0] } else { &pkg.name },
                    COLOR_RESET
                );
            }
            println!();
        }
    }

    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    if found == 0 {
        println!("  {}", t("Совпадений в каталоге репозиториев не найдено.", "No matching packages found."));
    } else {
        println!(
            "  {}: {}  •  {}",
            t("Найдено программ", "Packages found"),
            found,
            t("Для установки выполните: sudo wpm install <имя>", "To install run: sudo wpm install <name>")
        );
    }
    println!();

    0
}
