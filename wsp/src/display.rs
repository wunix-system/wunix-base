//! Модуль отображения и терминального вывода WSP.
//!
//! Обеспечивает чистый, строгий и информативный вывод в едином
//! стандарте системных утилит Wunix с поддержкой интернационализации t().

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

/// Выводит информацию о версии программы в стандартном формате
pub fn print_version() {
    println!("wsp (Wunix System Package) 1.0.1");
}

/// Выводит строку с выровненным полем вида `  Ключ:      Значение`
pub fn print_field(label: &str, value: &str) {
    println!("  {:<22} {}", label, value);
}

/// Выводит информационное сообщение утилиты
pub fn print_info(msg: &str) {
    println!("{}: {}", t("wsp", "wsp"), msg);
}

/// Выводит предупреждение
pub fn print_warning(msg: &str) {
    println!("{}{} {}{}", COLOR_YELLOW, t("wsp: предупреждение:", "wsp: warning:"), msg, COLOR_RESET);
}

/// Выводит ошибку
pub fn print_error(msg: &str) {
    println!("{}{} {}{}", COLOR_RED, t("wsp: ошибка:", "wsp: error:"), msg, COLOR_RESET);
}

/// Выводит стандартную справку по использованию утилиты
pub fn print_help() {
    println!(
        "{}",
        t(
            "Использование: wsp [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
             Диспетчер системных пакетов и обновлений ядра Wunix.\n\n\
             Команды:\n  \
               audit                    Полный аудит ядра, файлов и безопасности\n  \
               check                    Быстрая проверка наличия обновлений базы\n  \
               update                   Синхронизация индекса системного репозитория\n  \
               upgrade                  Обновление ядра и базовой системы\n  \
               list                     Список установленных системных компонентов\n  \
               info ПАКЕТ               Подробная информация о компоненте\n  \
               install ПАКЕТ            Установка системного компонента\n  \
               kernel [list|rollback]   Управление версиями ядра в загрузчике\n  \
               rollback                 Откат последней системной транзакции\n\n\
             Опции:\n  \
               -c, --critical           Работать только с критическими пакетами базы\n  \
               -y, --yes                Автоматическое подтверждение всех операций\n  \
               -v, --verbose, --verify  Подробный вывод и проверка контрольных сумм файлов\n  \
               -q, --quiet              Минимальный вывод сообщений\n  \
               -f, --files              Отобразить список файлов пакета (для wsp info)\n  \
                   --dry-run            Режим симуляции без изменения файлов на диске\n  \
                   --json               Вывод результатов в формате JSON (для wsp check)\n  \
               -h, --help               Показать эту справку и выйти\n  \
               -V, --version            Вывести информацию о версии и выйти",
            "Usage: wsp [OPTIONS]... [COMMAND] [ARGS]...\n\
             System package and kernel update manager for Wunix OS.\n\n\
             Commands:\n  \
               audit                    Full audit of kernel, files, and security\n  \
               check                    Fast check for available system updates\n  \
               update                   Synchronize system repository index\n  \
               upgrade                  Apply kernel and base system upgrades\n  \
               list                     List installed base system components\n  \
               info PACKAGE             Detailed information about a component\n  \
               install PACKAGE          Install a system package\n  \
               kernel [list|rollback]   Manage kernel versions in bootloader\n  \
               rollback                 Rollback last system transaction\n\n\
             Options:\n  \
               -c, --critical           Target critical base packages only\n  \
               -y, --yes                Assume yes for all prompts\n  \
               -v, --verbose, --verify  Verbose output and verify file checksums\n  \
               -q, --quiet              Suppress non-essential messages\n  \
               -f, --files              Show installed files list (for wsp info)\n  \
                   --dry-run            Simulation mode without modifying disk\n  \
                   --json               Output in JSON format (for wsp check)\n  \
               -h, --help               Display this help and exit\n  \
               -V, --version            Output version information and exit"
        )
    );
}
