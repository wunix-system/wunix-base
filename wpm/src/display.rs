//! Модуль отображения и текстового вывода WPM.
//!
//! Чистый, строгий и удобный системный интерфейс без посторонних значков и эмодзи.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{print, println, read_line, t};

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";

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

#[repr(C)]
struct Statfs {
    f_type: u64,
    f_bsize: u64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_fsid: [u64; 2],
    f_namelen: u64,
    f_frsize: u64,
    f_flags: u64,
    f_spare: [u64; 4],
}

pub fn get_system_used_bytes() -> u64 {
    let mut stat = core::mem::MaybeUninit::<Statfs>::uninit();
    let ret = libwunix::statfs(b"/\0", stat.as_mut_ptr() as *mut u8);
    if ret >= 0 {
        let s = unsafe { stat.assume_init() };
        if s.f_bsize > 0 && s.f_blocks >= s.f_bfree {
            let used = (s.f_blocks - s.f_bfree) * s.f_bsize;
            if used > 0 {
                return used;
            }
        }
    }
    // Базовый вес системы по умолчанию при невозможности опросить файловую систему
    65_000_000
}

pub fn print_disk_space_change(download_size: u64, installed_size: u64, is_install: bool) {
    let current_bytes = get_system_used_bytes();
    let final_bytes = if is_install {
        current_bytes.saturating_add(installed_size)
    } else {
        current_bytes.saturating_sub(installed_size)
    };

    println!("\n{}:", t("Изменение дискового пространства", "Disk space summary"));
    if is_install {
        println!("  {:<26} {}", t("Размер загрузки:", "Download size:"), format_size(download_size));
        println!("  {:<26} +{}", t("Будет занято на диске:", "Space required:"), format_size(installed_size));
    } else {
        println!("  {:<26} -{}", t("Будет освобождено на диске:", "Space freed:"), format_size(installed_size));
    }
    println!("  {:<26} {}", t("Текущий объем системы:", "Current system size:"), format_size(current_bytes));
    println!("  {:<26} {}", t("Итоговый объем системы:", "Final system size:"), format_size(final_bytes));
    println!();
}

pub fn confirm_prompt(prompt: &str, default_yes: bool) -> bool {
    print!("{}", prompt);
    let mut buf = [0u8; 64];
    match read_line(&mut buf) {
        Some(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                default_yes
            } else {
                trimmed.eq_ignore_ascii_case("y")
                    || trimmed.eq_ignore_ascii_case("yes")
                    || trimmed == "д"
                    || trimmed == "Д"
                    || trimmed.eq_ignore_ascii_case("да")
            }
        }
        None => false,
    }
}

pub fn print_version() {
    println!("wpm (Wunix Package Manager) 1.1.0 (x86_64)");
    println!("  {:<18} {}", t("Платформа:", "Platform:"), "Wunix OS");
    println!("  {:<18} {}", t("Назначение:", "Scope:"), t("Пользовательские программы и утилиты", "User applications and tools"));
    println!("  {:<18} {}", t("Формат пакетов:", "Format:"), "Wunix Package (.wp)");
    println!("  {:<18} {}", t("Конфигурация:", "Config:"), "/etc/wpm/wpm.conf");
    println!("  {:<18} {}", t("Источники:", "Sources:"), "/etc/wpm/sources.conf");
    println!("  {:<18} {}", t("Кэш пакетов:", "Cache:"), "/var/cache/wpm/");
}

pub fn print_help() {
    println!("{}", t(
        "Использование: wpm [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
         Менеджер пользовательских программ и пакетов .wp в ОС Wunix.\n\n\
         Команды:\n\
           search <запрос>          Поиск программ в каталоге репозиториев\n\
           install <пакет>...       Установка программ из локального кэша или файла .wp\n\
           remove <пакет>...        Удаление установленных программ\n\
           upgrade                  Обновление установленных программ\n\
           list                     Список установленных программ\n\
           info <пакет>             Подробная информация о программе\n\
           repo [list|add|rm]       Управление источниками репозиториев\n\
           update                   Синхронизация индексов репозиториев\n\
           clean                    Очистка кэша /var/cache/wpm\n\n\
         Опции:\n\
           -v, -V, --version        Показать версию утилиты\n\
           -h, --help               Показать эту справку\n\
           -y, --yes                Подтверждать операции без запроса\n\
           -d, --verbose            Подробный вывод процесса распаковки\n\
           --ignore-critical-unsafe Игнорировать защитный барьер безопасности WSP",
        "Usage: wpm [OPTIONS]... [COMMAND] [ARGS]...\n\
         User package manager for Wunix OS (.wp).\n\n\
         Commands:\n\
           search <query>           Search available applications\n\
           install <pkg>...         Install software from local cache or .wp file\n\
           remove <pkg>...          Remove installed software\n\
           upgrade                  Upgrade installed software\n\
           list                     List installed software\n\
           info <pkg>               Show details for a package\n\
           repo [list|add|rm]       Manage repository sources\n\
           update                   Synchronize repository indexes\n\
           clean                    Clean /var/cache/wpm\n\n\
         Options:\n\
           -v, -V, --version        Show version information\n\
           -h, --help               Show this help message\n\
           -y, --yes                Do not prompt for confirmation\n\
           -d, --verbose            Verbose extraction logs\n\
           --ignore-critical-unsafe Override WSP base security check"
    ));
}

pub fn print_access_denied(operation: &str, uid: u32) {
    println!("{}{}wpm: ошибка доступа:{} для операции '{}' требуются права администратора (root).", COLOR_BOLD, COLOR_RED, COLOR_RESET, operation);
    println!("  Текущий uid: {}", uid);
    println!("  Запустите команду через sudo: sudo wpm {} ...", operation);
}

pub fn print_wsp_blocked() {
    println!("{}{}wpm: внимание:{} обнаружены критические обновления базовой системы WSP.", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET);
    println!("  Установка программ временно заблокирована для сохранения стабильности ядра.");
    println!("  1. Обновите базовую систему: sudo wsp upgrade --critical");
    println!("  2. Перезагрузитесь: reboot");
    println!("  (для принудительной установки укажите: --ignore-critical-unsafe)");
}

pub fn print_warning(msg: &str) {
    println!("{}{}wpm: предупреждение:{} {}", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET, msg);
}

pub fn print_step(current: usize, total: usize, desc: &str, status_ok: bool) {
    let status = if status_ok {
        format!("{}[OK]{}", COLOR_GREEN, COLOR_RESET)
    } else {
        format!("{}[FAIL]{}", COLOR_RED, COLOR_RESET)
    };
    println!("  [{}/{}] {:<48} {}", current, total, desc, status);
}

pub fn print_info(msg: &str) {
    println!("wpm: {}", msg);
}

pub fn print_error(msg: &str) {
    println!("{}{}wpm: ошибка:{} {}", COLOR_BOLD, COLOR_RED, COLOR_RESET, msg);
}
