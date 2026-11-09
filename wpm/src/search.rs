//! Модуль поиска прикладных программ WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::println;

use crate::display::{COLOR_GREEN, COLOR_RESET};
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
    ]
}

pub fn run_search(query: &str) -> i32 {
    let catalog = get_wpm_catalog();
    let installed = load_installed_apps();

    println!("Поиск программ по запросу: \"{}\"", query);
    println!();

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
                "[доступен для установки]".to_string()
            };

            println!("  {} (v{}) [{}]", pkg.name, pkg.version, pkg.category);
            println!("    {}", pkg.description);

            if !pkg.aliases.is_empty() {
                let alias_list = pkg.aliases.join(", ");
                println!("    Алиасы команд: {}", alias_list);
            }

            println!(
                "    Размер: {} | Статус: {}",
                pkg.size,
                status_badge
            );

            if !is_installed {
                println!(
                    "    Установка: sudo wpm install {}",
                    if !pkg.aliases.is_empty() { &pkg.aliases[0] } else { &pkg.name }
                );
            }
            println!();
        }
    }

    if found == 0 {
        if crate::install::is_system_package(query) {
            println!("  '{}' является системным компонентом Wunix (WSP).", query);
            println!("  Для просмотра системного компонента используйте: wsp info {}", query);
            println!("  Для установки: sudo wsp install {}", query);
        } else {
            println!("  Совпадений в каталоге программ не найдено.");
        }
    } else {
        println!("Найдено программ: {} (для установки выполните: sudo wpm install <имя>)", found);
    }

    0
}
