//! Модуль просмотра установленных программ WPM.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};

use crate::display::{print_error, COLOR_BOLD, COLOR_CYAN, COLOR_DIM, COLOR_GREEN, COLOR_RESET};
use crate::install::load_installed_apps;
use crate::search::get_wpm_catalog;

pub fn run_list() -> i32 {
    let apps = load_installed_apps();

    println!();
    println!("  {}{}◆ WPM ── Реестр установленных пользовательских программ{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);

    if apps.is_empty() {
        println!("  {}", t("Установленных пользовательских программ пока нет.", "No user applications installed yet."));
        println!("  {}", t("Используйте 'wpm search <запрос>' для поиска программ.", "Use 'wpm search <query>' to browse available software."));
    } else {
        for a in &apps {
            println!("  {}● {}{} • v{}", COLOR_BOLD, a.name, COLOR_RESET, a.version);
            if !a.description.is_empty() {
                println!("    {}", a.description);
            }
            if !a.files.is_empty() {
                let bin = a.files.iter().find(|f| f.starts_with("/bin/"));
                if let Some(b) = bin {
                    println!("    {}: {}", t("Исполняемый файл", "Binary"), b);
                }
                println!("    {}: {}", t("Всего компонентов", "Total files"), a.files.len());
            }
            println!();
        }
    }

    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}: {}", t("Всего установленных программ", "Total installed applications"), apps.len());
    println!();

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
        print_error(&format!("{} '{}'", t("программа не найдена в каталоге WPM", "application not found in WPM catalog"), name));
        return 1;
    }

    println!();
    println!("  {}{}◆ WPM ── Карточка программы '{}'{}", COLOR_BOLD, COLOR_GREEN, name, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);

    if let Some(a) = inst_match {
        println!("  {:<18} {}", t("Имя:", "Name:"), a.name);
        println!("  {:<18} {}", t("Версия:", "Version:"), a.version);
        println!("  {:<18} {}{}{}", t("Статус:", "Status:"), COLOR_GREEN, t("Установлена в системе", "Installed"), COLOR_RESET);
        if !a.description.is_empty() {
            println!("  {:<18} {}", t("Описание:", "Description:"), a.description);
        }
        if !a.files.is_empty() {
            println!("  {}:", t("Установленные файлы", "Installed files"));
            for f in &a.files {
                println!("    ↳ {}", f);
            }
        }
    } else if let Some(p) = cat_match {
        println!("  {:<18} {}", t("Имя:", "Name:"), p.name);
        println!("  {:<18} {}", t("Версия:", "Version:"), p.version);
        println!("  {:<18} {}", t("Категория:", "Category:"), p.category);
        println!("  {:<18} {}", t("Описание:", "Description:"), p.description);
        println!("  {:<18} {}", t("Размер:", "Size:"), p.size);
        println!("  {:<18} {}{}{}", t("Статус:", "Status:"), COLOR_CYAN, t("Доступна в репозитории", "Available in repository"), COLOR_RESET);
        println!();
        println!("  {}: sudo wpm install {}", t("Команда для установки", "Install command"), name);
    }

    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!();

    0
}
