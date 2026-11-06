//! Модуль отображения и терминального вывода WPM.
//!
//! Обеспечивает строгий и удобный вывод в едином стандарте Wunix CLI
//! с двуязычной локализацией t().

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{println, t};

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
#[allow(dead_code)]
pub const COLOR_CYAN: &str = "\x1b[36m";
#[allow(dead_code)]
pub const COLOR_GRAY: &str = "\x1b[90m";

/// Форматирует размер в байтах в человекочитаемый вид
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

/// Выводит информацию о версии программы в стандартном формате
pub fn print_version() {
    println!("wpm (Wunix Package Manager) 1.0.1");
}

/// Выводит строку с выровненным полем вида `  Ключ:      Значение`
pub fn print_field(label: &str, value: &str) {
    println!("  {:<22} {}", label, value);
}

/// Выводит информационное сообщение утилиты
pub fn print_info(msg: &str) {
    println!("{}: {}", t("wpm", "wpm"), msg);
}

/// Выводит предупреждение
pub fn print_warning(msg: &str) {
    println!("{}{} {}{}", COLOR_YELLOW, t("wpm: предупреждение:", "wpm: warning:"), msg, COLOR_RESET);
}

/// Выводит ошибку
pub fn print_error(msg: &str) {
    println!("{}{} {}{}", COLOR_RED, t("wpm: ошибка:", "wpm: error:"), msg, COLOR_RESET);
}

/// Выводит справку по использованию утилиты
pub fn print_help() {
    println!(
        "{}",
        t(
            "Использование: wpm [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
             Диспетчер пользовательских программ и прикладных пакетов Wunix.\n\n\
             Команды:\n  \
               update                   Синхронизация списков пакетов репозиториев\n  \
               search ЗАПРОС            Поиск программ в настроенных репозиториях\n  \
               install ПАКЕТ...         Установка пользовательских программ\n  \
               remove ПАКЕТ...          Удаление установленных программ\n  \
               upgrade                  Обновление установленных программ\n  \
               list                     Список установленных пользовательских программ\n  \
               info ПАКЕТ               Подробная информация о программе\n  \
               repo [list|add|rm|ping]  Управление источниками репозиториев\n  \
               clean                    Очистка кэша загруженных пакетов\n\n\
             Опции:\n  \
               -y, --yes                Автоматическое подтверждение установки\n  \
               -q, --quiet              Минимальный вывод сообщений\n  \
               -v, --verbose            Подробный вывод действий\n  \
                   --purge              Удалить программу вместе с конфигурациями\n  \
                   --ignore-critical-unsafe  Игнорировать требование обязательных обновлений базы WSP\n  \
               -h, --help               Показать эту справку и выйти\n  \
               -V, --version            Вывести информацию о версии и выйти",
            "Usage: wpm [OPTIONS]... [COMMAND] [ARGS]...\n\
             Package manager for user applications and developer tools in Wunix OS.\n\n\
             Commands:\n  \
               update                   Synchronize package lists from repositories\n  \
               search QUERY             Search available applications in repositories\n  \
               install PACKAGE...       Install user applications\n  \
               remove PACKAGE...        Remove installed applications\n  \
               upgrade                  Upgrade all installed applications\n  \
               list                     List installed user applications\n  \
               info PACKAGE             Detailed information about a package\n  \
               repo [list|add|rm|ping]  Manage repository sources\n  \
               clean                    Clean downloaded packages cache\n\n\
             Options:\n  \
               -y, --yes                Assume yes for confirmation prompts\n  \
               -q, --quiet              Suppress non-essential messages\n  \
               -v, --verbose            Verbose progress output\n  \
                   --purge              Remove application along with configuration files\n  \
                   --ignore-critical-unsafe  Ignore required base system updates check\n  \
               -h, --help               Display this help and exit\n  \
               -V, --version            Output version information and exit"
        )
    );
}
