//! Модуль отображения и текстового вывода WSP.
//!
//! Чистый, строгий и удобный системный интерфейс без посторонних значков и эмодзи.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};
use libpackwunix::sources::get_wsp_system_repo;

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";

pub fn print_version() {
    println!("wsp (Wunix System Package) 1.1.0 (x86_64)");
    println!("  {:<18} {}", t("Платформа:", "Platform:"), "Wunix OS (Long Mode)");
    println!("  {:<18} {}", t("Назначение:", "Scope:"), t("Ядро и базовые системные библиотеки Ring 3", "Monolithic kernel and base runtime"));
    println!("  {:<18} {}", t("Формат пакетов:", "Format:"), "Wunix Package (.wp)");
    println!("  {:<18} {} {}", t("Репозиторий:", "Repository:"), get_wsp_system_repo(), t("[встроенный, защищен]", "[builtin, immutable]"));
    println!("  {:<18} {}", t("Конфигурация:", "Config:"), "/etc/wsp/wsp.conf");
}

pub fn print_help() {
    println!("{}", t(
        "Использование: wsp [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
         Системный диспетчер ядра и базовых компонентов .wp в ОС Wunix.\n\n\
         Команды:\n\
           audit                    Полный аудит ядра, файлов и безопасности\n\
           check                    Быстрая проверка наличия обновлений ядра\n\
           upgrade                  Применение обновлений ядра с созданием точки отката\n\
           list                     Список установленных системных компонентов\n\
           info <пакет>             Подробная информация о компоненте\n\
           install <пакет.wp>       Установка системного компонента из файла .wp\n\
           kernel [list|rollback]   Управление ядрами в каталоге /boot\n\
           rollback                 Откат к предыдущей версии ядра\n\
           update                   Синхронизация индекса системного репозитория\n\n\
         Опции:\n\
           -v, -V, --version        Показать версию утилиты\n\
           -h, --help               Показать эту справку\n\
           -c, --critical           Работать только с критическими пакетами базы\n\
           -y, --yes                Подтверждать операции без запроса\n\
           -d, --verbose            Подробный вывод процесса проверки файлов\n\
           --dry-run                Режим симуляции без изменения файлов на диске",
        "Usage: wsp [OPTIONS]... [COMMAND] [ARGS]...\n\
         Kernel and base system package manager for Wunix OS (.wp).\n\n\
         Commands:\n\
           audit                    Full audit of kernel and base files\n\
           check                    Fast check for available kernel updates\n\
           upgrade                  Upgrade kernel and base system\n\
           list                     List installed base components\n\
           info <pkg>               Detailed component info\n\
           install <pkg.wp>         Install system package from .wp file\n\
           kernel [list|rollback]   Manage kernel versions in /boot\n\
           rollback                 Rollback to fallback kernel\n\
           update                   Synchronize system repository index\n\n\
         Options:\n\
           -v, -V, --version        Show version information\n\
           -h, --help               Show this help message\n\
           -c, --critical           Target critical base components only\n\
           -y, --yes                Do not prompt for confirmation\n\
           -d, --verbose            Verbose execution details\n\
           --dry-run                Simulation mode without disk writes"
    ));
}

pub fn print_access_denied(operation: &str, uid: u32) {
    println!("{}{}wsp: ошибка доступа:{} для операции '{}' требуются права администратора (root).", COLOR_BOLD, COLOR_RED, COLOR_RESET, operation);
    println!("  Текущий uid: {}", uid);
    println!("  Запустите команду через sudo: sudo wsp {} ...", operation);
}

pub fn print_step(current: usize, total: usize, desc: &str, status_ok: bool) {
    let status = if status_ok {
        format!("{}[OK]{}", COLOR_GREEN, COLOR_RESET)
    } else {
        format!("{}[FAIL]{}", COLOR_RED, COLOR_RESET)
    };
    println!("  [{}/{}] {:<48} {}", current, total, desc, status);
}

#[allow(dead_code)]
pub fn print_info(msg: &str) {
    println!("wsp: {}", msg);
}

pub fn print_error(msg: &str) {
    println!("{}{}wsp: ошибка:{} {}", COLOR_BOLD, COLOR_RED, COLOR_RESET, msg);
}
