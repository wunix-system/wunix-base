//! Модуль отображения и терминального вывода WPM.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{println, t};

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
#[allow(dead_code)]
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_CYAN: &str = "\x1b[36m";
pub const COLOR_GRAY: &str = "\x1b[90m";

#[allow(dead_code)]
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} {}", bytes, t("Б", "B"))
    } else if bytes < 1024 * 1024 {
        let kb = (bytes as f64) / 1024.0;
        format!("{:.1} {}", kb, t("КиБ", "KiB"))
    } else {
        let mb = (bytes as f64) / (1024.0 * 1024.0);
        format!("{:.2} {}", mb, t("МиБ", "MiB"))
    }
}

pub fn print_card_header(title: &str) {
    println!("{}{}┌── [ {} ] {}────────────────────────────────────────{}", 
        COLOR_BOLD, COLOR_CYAN, title, COLOR_GRAY, COLOR_RESET);
}

pub fn print_card_row(label: &str, value: &str) {
    println!("{}│{} {:<22} {}{}{}", 
        COLOR_GRAY, COLOR_RESET, label, COLOR_BOLD, value, COLOR_RESET);
}

#[allow(dead_code)]
pub fn print_card_separator() {
    println!("{}├───────────────────────────────────────────────────────────────────────────────┤{}", 
        COLOR_GRAY, COLOR_RESET);
}

pub fn print_card_footer() {
    println!("{}└───────────────────────────────────────────────────────────────────────────────┘{}", 
        COLOR_GRAY, COLOR_RESET);
}

pub fn print_help() {
    println!("{}{}wpm - Пользовательский менеджер пакетов Wunix Package Manager (WPM){}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("{}", t("Использование: wpm <команда> [аргументы] [флаги]", "Usage: wpm <command> [args] [flags]"));
    println!();
    println!("{}", t("Команды:", "Commands:"));
    println!("  {}update{}             {}  {}", COLOR_BOLD, COLOR_RESET, t("Синхронизация списков пакетов с репозиториев", "Sync package lists from repositories"), COLOR_GRAY);
    println!("  {}search <запрос>{}    {}  {}", COLOR_BOLD, COLOR_RESET, t("Поиск программ в репозиториях", "Search software in repositories"), COLOR_GRAY);
    println!("  {}install <пакет>...{} {}  {}", COLOR_BOLD, COLOR_RESET, t("Установка прикладных программ", "Install user applications"), COLOR_GRAY);
    println!("  {}remove <пакет>...{}  {}  {}", COLOR_BOLD, COLOR_RESET, t("Удаление программ", "Remove user applications"), COLOR_GRAY);
    println!("  {}upgrade{}            {}  {}", COLOR_BOLD, COLOR_RESET, t("Обновление установленных программ", "Upgrade all installed applications"), COLOR_GRAY);
    println!("  {}list{}               {}  {}", COLOR_BOLD, COLOR_RESET, t("Список установленных программ", "List installed programs"), COLOR_GRAY);
    println!("  {}info <пакет>{}       {}  {}", COLOR_BOLD, COLOR_RESET, t("Подробная информация о программе", "Detailed package information"), COLOR_GRAY);
    println!("  {}repo{}               {}  {}", COLOR_BOLD, COLOR_RESET, t("Управление источниками (list, add, remove)", "Manage repository sources"), COLOR_GRAY);
    println!("  {}clean{}              {}  {}", COLOR_BOLD, COLOR_RESET, t("Очистка локального кэша пакетов", "Clean local package cache"), COLOR_GRAY);
    println!();
    println!("{}", t("Флаги:", "Flags:"));
    println!("  -y, --yes                    {}", t("Автоматическое подтверждение установки", "Assume yes for prompts"));
    println!("  --ignore-critical-unsafe     {}", t("Игнорировать критические предупреждения базы WSP", "Bypass WSP critical update check"));
    println!("  --purge                      {}", t("Удалить вместе с файлами конфигурации", "Purge configuration files"));
    println!("  -h, --help                   {}", t("Показать эту справку", "Show this help message"));
}
