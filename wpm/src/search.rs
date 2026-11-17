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
            name: "archiveutils".to_string(),
            aliases: alloc::vec!["ar".to_string(), "cpio".to_string(), "unrar".to_string()],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Пользовательские утилиты работы со специальными архивными контейнерами (ar, cpio, unrar)".to_string(),
            size: "16.5 KiB".to_string(),
        },
        CatalogPackage {
            name: "curl".to_string(),
            aliases: alloc::vec![],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Сетевой клиент HTTP/HTTPS для передачи данных и API".to_string(),
            size: "2.35 MiB".to_string(),
        },
        CatalogPackage {
            name: "git".to_string(),
            aliases: alloc::vec![],
            version: "2.43.0".to_string(),
            category: "applications".to_string(),
            description: "Быстрая распределенная система управления версиями файлов Git".to_string(),
            size: "1.84 MiB".to_string(),
        },
        CatalogPackage {
            name: "gzip".to_string(),
            aliases: alloc::vec!["gunzip".to_string()],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Утилиты потокового сжатия и распаковки gzip (gzip, gunzip)".to_string(),
            size: "14.9 KiB".to_string(),
        },
        CatalogPackage {
            name: "nano".to_string(),
            aliases: alloc::vec![],
            version: "7.2.0".to_string(),
            category: "applications".to_string(),
            description: "Текстовый редактор GNU nano для Wunix".to_string(),
            size: "846.3 KiB".to_string(),
        },
        CatalogPackage {
            name: "openssh".to_string(),
            aliases: alloc::vec!["ssh".to_string(), "sshd".to_string(), "scp".to_string()],
            version: "9.8.0".to_string(),
            category: "applications".to_string(),
            description: "Комплекс сетевого безопасного доступа OpenSSH (ssh, sshd, scp)".to_string(),
            size: "12.1 MiB".to_string(),
        },
        CatalogPackage {
            name: "tar".to_string(),
            aliases: alloc::vec![],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Утилита работы со стандартными архивами tar".to_string(),
            size: "9.2 KiB".to_string(),
        },
        CatalogPackage {
            name: "tinycc".to_string(),
            aliases: alloc::vec!["tcc".to_string(), "cc".to_string()],
            version: "0.9.27".to_string(),
            category: "programs-dev".to_string(),
            description: "Быстрый и легковесный компилятор Си (tcc) для Wunix".to_string(),
            size: "244.5 KiB".to_string(),
        },
        CatalogPackage {
            name: "wc".to_string(),
            aliases: alloc::vec![],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Пользовательская программа wc для подсчета строк, слов и байтов".to_string(),
            size: "6.1 KiB".to_string(),
        },
        CatalogPackage {
            name: "zip".to_string(),
            aliases: alloc::vec!["unzip".to_string()],
            version: "1.0.0".to_string(),
            category: "applications".to_string(),
            description: "Утилиты сжатия и распаковки ZIP архивов (zip, unzip)".to_string(),
            size: "17.1 KiB".to_string(),
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
