//! Модуль поиска прикладных программ WPM.
//!
//! Динамически считывает индекс репозитория packages.json из кэша и системных
//! каталогов, сопоставляет запросы с именами, алиасами и описаниями программ.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::println;

use crate::display::{COLOR_GREEN, COLOR_RESET};
use crate::install::{detect_pkg_version, load_installed_apps};
use crate::progress::format_bytes;
use crate::unpack::read_file_to_vec;

#[derive(Debug, Clone)]
pub struct CatalogPackage {
    pub name: String,
    pub aliases: Vec<String>,
    pub version: String,
    pub category: String,
    pub description: String,
    pub size: String,
}

pub fn get_aliases_for(name: &str) -> Vec<String> {
    match name {
        "tinycc" => alloc::vec!["tcc".to_string(), "cc".to_string()],
        "archiveutils" => alloc::vec!["ar".to_string(), "cpio".to_string(), "unrar".to_string()],
        "zip" => alloc::vec!["unzip".to_string()],
        "gzip" => alloc::vec!["gunzip".to_string()],
        "openssh" => alloc::vec!["ssh".to_string(), "sshd".to_string(), "scp".to_string()],
        _ => Vec::new(),
    }
}

pub fn load_wpm_catalog() -> Vec<CatalogPackage> {
    load_wpm_catalog_from("/")
}

pub fn load_wpm_catalog_from(root: &str) -> Vec<CatalogPackage> {
    let r = if root.is_empty() || root == "/" { "" } else { root.trim_end_matches('/') };
    let candidate_paths = [
        format!("{}/var/lib/wpm/repo/apps/x86_64/packages.json", r),
        format!("{}/var/cache/wpm/packages.json", r),
        format!("{}/repo/apps/x86_64/packages.json", r),
        format!("{}/repositories/apps/x86_64/packages.json", r),
        "/var/lib/wpm/repo/apps/x86_64/packages.json".to_string(),
        "/var/cache/wpm/packages.json".to_string(),
    ];

    for path in &candidate_paths {
        if let Ok(data) = read_file_to_vec(path) {
            if let Ok(s) = core::str::from_utf8(&data) {
                let pkgs = libpackwunix::sources::parse_repo_index(s);
                if !pkgs.is_empty() {
                    return pkgs
                        .into_iter()
                        .map(|p| {
                            let aliases = get_aliases_for(&p.name);
                            let category = if p.name == "tinycc" {
                                "programs-dev".to_string()
                            } else {
                                "applications".to_string()
                            };
                            CatalogPackage {
                                name: p.name,
                                aliases,
                                version: p.version.to_string(),
                                category,
                                description: p.description,
                                size: format_bytes(p.size),
                            }
                        })
                        .collect();
                }
            }
        }
    }

    get_default_wpm_catalog()
}

pub fn get_default_wpm_catalog() -> Vec<CatalogPackage> {
    alloc::vec![
        CatalogPackage {
            name: "archiveutils".to_string(),
            aliases: alloc::vec!["ar".to_string(), "cpio".to_string(), "unrar".to_string()],
            version: detect_pkg_version("archiveutils"),
            category: "applications".to_string(),
            description: "Пользовательские утилиты работы со специальными архивными контейнерами (ar, cpio, unrar)".to_string(),
            size: "16.5 КиБ".to_string(),
        },
        CatalogPackage {
            name: "curl".to_string(),
            aliases: alloc::vec![],
            version: detect_pkg_version("curl"),
            category: "applications".to_string(),
            description: "Сетевой клиент HTTP/HTTPS для передачи данных и API".to_string(),
            size: "2.35 МиБ".to_string(),
        },
        CatalogPackage {
            name: "git".to_string(),
            aliases: alloc::vec![],
            version: "2.43.0".to_string(),
            category: "applications".to_string(),
            description: "Быстрая распределенная система управления версиями файлов Git".to_string(),
            size: "1.84 МиБ".to_string(),
        },
        CatalogPackage {
            name: "gzip".to_string(),
            aliases: alloc::vec!["gunzip".to_string()],
            version: detect_pkg_version("gzip"),
            category: "applications".to_string(),
            description: "Утилиты потокового сжатия и распаковки gzip (gzip, gunzip)".to_string(),
            size: "14.9 КиБ".to_string(),
        },
        CatalogPackage {
            name: "nano".to_string(),
            aliases: alloc::vec![],
            version: "7.2.0".to_string(),
            category: "applications".to_string(),
            description: "Текстовый редактор GNU nano для Wunix".to_string(),
            size: "846.3 КиБ".to_string(),
        },
        CatalogPackage {
            name: "openssh".to_string(),
            aliases: alloc::vec!["ssh".to_string(), "sshd".to_string(), "scp".to_string()],
            version: "9.8.0".to_string(),
            category: "applications".to_string(),
            description: "Комплекс сетевого безопасного доступа OpenSSH (ssh, sshd, scp)".to_string(),
            size: "12.1 МиБ".to_string(),
        },
        CatalogPackage {
            name: "tar".to_string(),
            aliases: alloc::vec![],
            version: detect_pkg_version("tar"),
            category: "applications".to_string(),
            description: "Утилита работы со стандартными архивами tar".to_string(),
            size: "9.2 КиБ".to_string(),
        },
        CatalogPackage {
            name: "tinycc".to_string(),
            aliases: alloc::vec!["tcc".to_string(), "cc".to_string()],
            version: "0.9.27".to_string(),
            category: "programs-dev".to_string(),
            description: "Быстрый и легковесный компилятор Си (tcc) для Wunix".to_string(),
            size: "244.5 КиБ".to_string(),
        },
        CatalogPackage {
            name: "wc".to_string(),
            aliases: alloc::vec![],
            version: detect_pkg_version("wc"),
            category: "applications".to_string(),
            description: "Пользовательская программа wc для подсчета строк, слов и байтов".to_string(),
            size: "6.1 КиБ".to_string(),
        },
        CatalogPackage {
            name: "zip".to_string(),
            aliases: alloc::vec!["unzip".to_string()],
            version: detect_pkg_version("zip"),
            category: "applications".to_string(),
            description: "Утилиты сжатия и распаковки ZIP архивов (zip, unzip)".to_string(),
            size: "17.1 КиБ".to_string(),
        },
    ]
}

pub fn run_search(query: &str) -> i32 {
    let catalog = load_wpm_catalog();
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
            println!("  Подсказка: выполните 'wpm update' для обновления каталога из репозитория.");
        }
    } else {
        println!("Найдено программ: {} (для установки выполните: sudo wpm install <имя>)", found);
    }

    0
}
