//! Модуль отображения и текстового вывода WSP.
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
        println!("  {:<26} {}", t("Размер пакетов:", "Download size:"), format_size(download_size));
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
    println!("wsp (Wunix System Package) 0.1-{} (x86_64)", env!("WUNIX_GIT_COMMIT"));
}

pub fn print_help() {
    println!("{}", t(
        "Использование: wsp [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
         Системный диспетчер ядра и базовых компонентов .wp в ОС Wunix.\n\n\
         Команды и короткие флаги (стиль pacman):\n\
           -S, install <пакет>...       Установка системного компонента\n\
           -Syu                         Синхронизация и обновление базы/ядра\n\
           -Sy, update                  Синхронизация индекса системного репозитория\n\
           -Su, upgrade                 Применение обновлений ядра и базовой системы\n\
           -Ss, check                   Проверка наличия обновлений базовой системы\n\
           -Si, info <пакет>            Подробная информация о компоненте\n\
           -Sc                          Очистка кэша /var/cache/wsp\n\
           -Q, list                     Список установленных системных компонентов\n\
           -Qi <пакет>                  Информация об установленном компоненте\n\
           -U <файл.wp>...              Установка из локального файла пакета\n\
           audit                        Полный аудит ядра, файлов и безопасности\n\
           kernel [list|rollback]       Управление ядрами в каталоге /boot\n\
           rollback                     Откат к предыдущей версии ядра\n\n\
         Опции:\n\
           -v, -V, --version            Показать версию утилиты\n\
           -h, --help                   Показать эту справку\n\
           -c, --critical               Работать только с критическими пакетами базы\n\
           -y, --yes, --noconfirm       Подтверждать операции без запроса\n\
           -d, --verbose                Подробный вывод процесса\n\
           -q, --quiet                  Минимальный вывод сообщений\n\
           --reinstall                  Переустановить уже установленные компоненты\n\
           --dry-run                    Режим симуляции без изменения файлов на диске",
        "Usage: wsp [OPTIONS]... [COMMAND] [ARGS]...\n\
         Kernel and base system package manager for Wunix OS (.wp).\n\n\
         Commands and short flags (pacman style):\n\
           -S, install <pkg>...         Install system component\n\
           -Syu                         Synchronize and upgrade kernel and base\n\
           -Sy, update                  Synchronize system repository index\n\
           -Su, upgrade                 Upgrade kernel and base system\n\
           -Ss, check                   Check for available base updates\n\
           -Si, info <pkg>              Detailed component info\n\
           -Sc                          Clean /var/cache/wsp\n\
           -Q, list                     List installed base components\n\
           -Qi <pkg>                    Show details of installed component\n\
           -U <file.wp>...              Install component from local package file\n\
           audit                        Full audit of kernel and base files\n\
           kernel [list|rollback]       Manage kernel versions in /boot\n\
           rollback                     Rollback to fallback kernel\n\n\
         Options:\n\
           -v, -V, --version            Show version information\n\
           -h, --help                   Show this help message\n\
           -c, --critical               Target critical base components only\n\
           -y, --yes, --noconfirm       Do not prompt for confirmation\n\
           -d, --verbose                Verbose execution details\n\
           -q, --quiet                  Quiet mode\n\
           --reinstall                  Reinstall already installed components\n\
           --dry-run                    Simulation mode without disk writes"
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
