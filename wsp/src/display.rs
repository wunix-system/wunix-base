//! Модуль отображения и терминального вывода WSP.
//!
//! Отвечает за человекочитаемый вывод: рамки, таблицы пакетов,
//! цветную индикацию критических статусов и локализованные сообщения.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{println, t};

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_CYAN: &str = "\x1b[36m";
pub const COLOR_GRAY: &str = "\x1b[90m";

/// Форматирует размер в байтах в человекочитаемый вид (Байт, КиБ, МиБ)
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

/// Выводит заголовок информационной карточки
pub fn print_card_header(title: &str) {
    println!("{}{}┌── [ {} ] {}────────────────────────────────────────{}", 
        COLOR_BOLD, COLOR_CYAN, title, COLOR_GRAY, COLOR_RESET);
}

/// Выводит строку информационной карточки
pub fn print_card_row(label: &str, value: &str) {
    println!("{}│{} {:<22} {}{}{}", 
        COLOR_GRAY, COLOR_RESET, label, COLOR_BOLD, value, COLOR_RESET);
}

/// Выводит разделитель информационной карточки
pub fn print_card_separator() {
    println!("{}├───────────────────────────────────────────────────────────────────────────────┤{}", 
        COLOR_GRAY, COLOR_RESET);
}

/// Закрывает информационную карточку
pub fn print_card_footer() {
    println!("{}└───────────────────────────────────────────────────────────────────────────────┘{}", 
        COLOR_GRAY, COLOR_RESET);
}

/// Выводит баннер критического предупреждения безопасности
#[allow(dead_code)]
pub fn print_critical_alert(msg: &str) {
    println!("\n{}{}┌────────────────────────────────────────────────────────────────────────┐{}", COLOR_BOLD, COLOR_RED, COLOR_RESET);
    println!("{}│ [!] ВНИМАНИЕ: КРИТИЧЕСКИЙ СТАТУС БАЗОВОЙ СИСТЕМЫ                       │{}", COLOR_RED, COLOR_RESET);
    println!("{}│ {}│{}", COLOR_RED, format!("{:<70}", msg), COLOR_RESET);
    println!("{}{}└────────────────────────────────────────────────────────────────────────┘{}\n", COLOR_BOLD, COLOR_RED, COLOR_RESET);
}

/// Выводит справку по использованию утилиты
pub fn print_help() {
    println!("{}{}wsp - Системный менеджер пакетов Wunix System Package (WSP){}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("{}", t("Использование: wsp <команда> [флаги]", "Usage: wsp <command> [flags]"));
    println!();
    println!("{}", t("Команды:", "Commands:"));
    println!("  {}audit{}       {}  {}", COLOR_BOLD, COLOR_RESET, t("Полный аудит ядра, файлов и безопасности системы", "Full system audit of kernel, files and security"), COLOR_GRAY);
    println!("  {}check{}       {}  {}", COLOR_BOLD, COLOR_RESET, t("Быстрая проверка наличия системных обновлений", "Fast non-interactive check for updates"), COLOR_GRAY);
    println!("  {}update{}      {}  {}", COLOR_BOLD, COLOR_RESET, t("Синхронизация индекса системного репозитория", "Synchronize system repository index"), COLOR_GRAY);
    println!("  {}upgrade{}     {}  {}", COLOR_BOLD, COLOR_RESET, t("Применение обновлений ядра и базовой системы", "Upgrade kernel and base system components"), COLOR_GRAY);
    println!("  {}list{}        {}  {}", COLOR_BOLD, COLOR_RESET, t("Список установленных системных компонентов", "List installed system packages"), COLOR_GRAY);
    println!("  {}info <пакет>{} {}  {}", COLOR_BOLD, COLOR_RESET, t("Подробная информация о системном пакете", "Detailed package information"), COLOR_GRAY);
    println!("  {}install <пкт>{}   {}  {}", COLOR_BOLD, COLOR_RESET, t("Установка системного компонента / профиля", "Install system package or profile"), COLOR_GRAY);
    println!("  {}kernel{}      {}  {}", COLOR_BOLD, COLOR_RESET, t("Управление ядрами (list, rollback, clean)", "Kernel management: list, rollback, clean"), COLOR_GRAY);
    println!("  {}rollback{}    {}  {}", COLOR_BOLD, COLOR_RESET, t("Откат последней системной транзакции", "Rollback last system transaction"), COLOR_GRAY);
    println!();
    println!("{}", t("Флаги:", "Flags:"));
    println!("  -c, --critical   {}", t("Ограничиться только критическими пакетами базы", "Target critical packages only"));
    println!("  -y, --yes        {}", t("Автоматическое подтверждение всех действий", "Assume yes for all prompts"));
    println!("  -v, --verify     {}", t("Пофайловая сверка контрольных сумм SHA-256", "Verify file checksums on disk"));
    println!("  -h, --help       {}", t("Показать эту справку", "Show this help message"));
}
