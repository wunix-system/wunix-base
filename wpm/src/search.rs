//! Модуль поиска пакетов и программ во всех репозиториях WPM.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{println, t};

use crate::db::{load_installed_packages, load_unified_catalog_from};
use crate::display::{COLOR_GREEN, COLOR_RESET, COLOR_YELLOW};
use crate::progress::format_bytes;

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

pub fn run_search(query: &str) -> i32 {
    let catalog = load_unified_catalog_from("/");
    let installed = load_installed_packages();

    println!("{}: \"{}\"", t("Поиск пакетов по запросу", "Searching packages for"), query);
    println!();

    let mut found = 0;
    for entry in &catalog {
        let p = &entry.pkg;
        let aliases = get_aliases_for(&p.name);
        let match_name = p.name.contains(query);
        let match_alias = aliases.iter().any(|a| a.contains(query));
        let match_desc = p.description.contains(query);
        let match_chan = entry.channel.contains(query);

        if query.is_empty() || match_name || match_alias || match_desc || match_chan {
            found += 1;
            let is_installed = installed.iter().any(|a| a.name == p.name);

            let status_badge = if is_installed {
                format!("{}[установлен]{}", COLOR_GREEN, COLOR_RESET)
            } else {
                "[доступен для установки]".to_string()
            };

            let kind_tag = if p.is_meta {
                format!(" {}[метапакет]{}", COLOR_YELLOW, COLOR_RESET)
            } else {
                String::new()
            };

            println!("  {} (v{}) [{}/{}]{}", p.name, p.version, entry.channel, p.tier, kind_tag);
            if !p.description.is_empty() {
                println!("    {}", p.description);
            }
            if !aliases.is_empty() {
                println!("    Алиасы команд: {}", aliases.join(", "));
            }
            println!("    Размер: {} | Статус: {}", format_bytes(p.size), status_badge);
            if !is_installed {
                let cmd_name = if !aliases.is_empty() { &aliases[0] } else { &p.name };
                println!("    Установка: sudo wpm install {}", cmd_name);
            }
            println!();
        }
    }

    if found == 0 {
        println!("  {}", t("Совпадений в каталогах пакетов не найдено.", "No matching packages found."));
        println!("  {}", t("Подсказка: выполните 'sudo wpm update' для синхронизации репозиториев.", "Hint: run 'sudo wpm update' to synchronize repositories."));
    } else {
        println!("Найдено пакетов: {} (для установки выполните: sudo wpm install <имя>)", found);
    }

    0
}
