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
        CatalogPackage {
            name: "curl".to_string(),
            aliases: alloc::vec!["http".to_string()],
            version: "8.6.0".to_string(),
            category: "network".to_string(),
            description: "Сетевой клиент HTTP/HTTPS для передачи файлов и API".to_string(),
            size: "7.20 MiB".to_string(),
        },
        CatalogPackage {
            name: "nano".to_string(),
            aliases: alloc::vec!["editor".to_string()],
            version: "7.2.0".to_string(),
            category: "editors".to_string(),
            description: "Удобный консольный текстовый редактор GNU nano".to_string(),
            size: "830.0 KiB".to_string(),
        },
        CatalogPackage {
            name: "openssh".to_string(),
            aliases: alloc::vec!["ssh".to_string(), "sshd".to_string(), "scp".to_string()],
            version: "9.8.0".to_string(),
            category: "network".to_string(),
            description: "Комплекс сетевого безопасного доступа OpenSSH".to_string(),
            size: "2.10 MiB".to_string(),
        },
        CatalogPackage {
            name: "tar".to_string(),
            aliases: alloc::vec![],
            version: "1.0.0".to_string(),
            category: "archivers".to_string(),
            description: "Утилита работы со стандартными архивами tar".to_string(),
            size: "18.0 KiB".to_string(),
        },
        CatalogPackage {
            name: "zip".to_string(),
            aliases: alloc::vec!["unzip".to_string()],
            version: "1.0.0".to_string(),
            category: "archivers".to_string(),
            description: "Утилиты сжатия и распаковки ZIP архивов (zip, unzip)".to_string(),
            size: "35.0 KiB".to_string(),
        },
        CatalogPackage {
            name: "gzip".to_string(),
            aliases: alloc::vec!["gunzip".to_string()],
            version: "1.0.0".to_string(),
            category: "archivers".to_string(),
            description: "Утилиты потокового сжатия и распаковки gzip (gzip, gunzip)".to_string(),
            size: "33.0 KiB".to_string(),
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
        println!("  Совпадений в каталоге программ не найдено.");
    } else {
        println!("Найдено программ: {} (для установки выполните: sudo wpm install <имя>)", found);
    }

    0
}
