//! Модуль просмотра установленных программ WPM.

extern crate alloc;
use alloc::format;
use libwunix::println;

use crate::display::{print_error, COLOR_GREEN, COLOR_RESET};
use crate::install::load_installed_apps;
use crate::search::get_wpm_catalog;

pub fn run_list() -> i32 {
    let apps = load_installed_apps();

    println!("Установленные пользовательские программы (WPM):");
    println!();

    if apps.is_empty() {
        println!("  (нет установленных пользовательских программ)");
        println!("  Используйте 'wpm search <запрос>' для поиска программ.");
    } else {
        for a in &apps {
            println!("  {} (v{})", a.name, a.version);
            if !a.description.is_empty() {
                println!("    {}", a.description);
            }
            if !a.files.is_empty() {
                let bin = a.files.iter().find(|f| f.starts_with("/bin/"));
                if let Some(b) = bin {
                    println!("    Исполняемый файл: {}", b);
                }
                println!("    Файлов: {}", a.files.len());
            }
            println!();
        }
    }

    println!("Всего установленных программ: {}", apps.len());
    0
}

pub fn run_info(name: &str) -> i32 {
    let installed = load_installed_apps();
    let catalog = get_wpm_catalog();

    let canonical = match name {
        "tcc" => "tinycc",
        other => other,
    };

    let inst_match = installed.iter().find(|a| a.name == canonical || a.name == name);
    let cat_match = catalog.iter().find(|p| p.name == canonical || p.name == name || p.aliases.iter().any(|al| al == name));

    if inst_match.is_none() && cat_match.is_none() {
        print_error(&format!("программа '{}' не найдена в каталоге WPM", name));
        return 1;
    }

    println!("Информация о программе '{}':", name);

    if let Some(a) = inst_match {
        println!("  Имя:         {}", a.name);
        println!("  Версия:      {}", a.version);
        println!("  Статус:      {}[установлена в системе]{}", COLOR_GREEN, COLOR_RESET);
        if !a.description.is_empty() {
            println!("  Описание:    {}", a.description);
        }
        if !a.files.is_empty() {
            println!("  Файлы:");
            for f in &a.files {
                println!("    - {}", f);
            }
        }
    } else if let Some(p) = cat_match {
        println!("  Имя:         {}", p.name);
        println!("  Версия:      {}", p.version);
        println!("  Категория:   {}", p.category);
        println!("  Описание:    {}", p.description);
        println!("  Размер:      {}", p.size);
        println!("  Статус:      [доступна в репозитории]");
        println!();
        println!("  Установка: sudo wpm install {}", name);
    }

    0
}
