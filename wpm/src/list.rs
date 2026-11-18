//! Модуль просмотра установленных программ WPM.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};
use libpackwunix::version::Version;

use crate::display::{print_error, COLOR_GREEN, COLOR_RESET};
use crate::install::{load_installed_apps, load_installed_apps_from};
use crate::search::{load_wpm_catalog, load_wpm_catalog_from};

pub fn run_check(root_prefix: Option<&str>) -> i32 {
    let root_str = root_prefix.unwrap_or("/");
    let installed = load_installed_apps_from(root_str);
    let catalog = load_wpm_catalog_from(root_str);

    let mut count = 0;
    for app in &installed {
        let clean_ver = app.version.trim_start_matches(|c| c == 'v' || c == 'V');
        let app_ver = Version::parse(clean_ver).unwrap_or(Version::new(0, 1, 0));
        if let Some(cat_pkg) = catalog.iter().find(|p| p.name == app.name || p.aliases.iter().any(|a| a == &app.name)) {
            let cat_clean = cat_pkg.version.trim_start_matches(|c| c == 'v' || c == 'V');
            if let Ok(cat_ver) = Version::parse(cat_clean) {
                if cat_ver.is_newer_than(&app_ver) {
                    println!("- {:<16} v{} -> v{}", cat_pkg.name, app_ver, cat_ver);
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("{}", t("wpm: все установленные программы обновлены до актуальных версий.", "wpm: all installed programs are up to date."));
    }

    0
}

pub fn run_list() -> i32 {
    run_list_in("/")
}

pub fn run_list_in(root: &str) -> i32 {
    let apps = load_installed_apps_from(root);

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
    let catalog = load_wpm_catalog();

    let pkg_id = match name {
        "tcc" => "tinycc",
        other => other,
    };

    let inst_match = installed.iter().find(|a| a.name == pkg_id || a.name == name);
    let cat_match = catalog.iter().find(|p| p.name == pkg_id || p.name == name || p.aliases.iter().any(|al| al == name));

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
