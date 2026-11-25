//! Модуль отображения и текстового вывода единого WPM.

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
    println!("wpm (Wunix Package Manager) 0.1-{} (x86_64)", env!("WUNIX_GIT_COMMIT"));
}

pub fn print_help() {
    println!("{}", t(
        "Использование: wpm [ОПЦИИ]... [КОМАНДА] [АРГУМЕНТЫ]...\n\
         Единый пакетный менеджер ОС Wunix (системные, прикладные пакеты .wp и метапакеты).\n\n\
         Команды и короткие флаги (стиль pacman):\n\
           -S, install <пакет>...       Установка одного или нескольких пакетов\n\
           reinstall <пакет>...         Принудительная переустановка пакетов\n\
           -Syu                         Синхронизация и обновление всех пакетов\n\
           -Sy, update                  Синхронизация системного и пользовательских каталогов\n\
           -Su, upgrade                 Обновление установленных пакетов и ядра\n\
           -Ss, search <запрос>         Поиск пакетов во всех репозиториях\n\
           -Si, info <пакет>            Подробная информация о пакете\n\
           -Sc, clean                   Очистка локального кэша /var/cache/wpm\n\
           -R, remove <пакет>...        Удаление пользовательских пакетов\n\
           -Q, list                     Список установленных пакетов\n\
           -Qu, check, updates          Проверка доступных обновлений\n\
           -Qi <пакет>                  Информация об установленном пакете\n\
           -U <файл.wp>...              Установка из локального файла .wp\n\
           repo [list|add|rm|ping]      Управление пользовательскими репозиториями\n\
           audit [-v]                   Аудит целостности ядра и системных файлов\n\
           kernel [list|rollback|clean] Управление ядрами в /boot\n\
           rollback                     Откат к резервному ядру\n\n\
         Опции:\n\
           -v, -V, --version            Показать версию утилиты\n\
           -h, --help                   Показать эту справку\n\
           -c, --critical               Работать только с критическими системными пакетами\n\
           -u, --upgradable             Показать доступные обновления\n\
           -y, --yes, --noconfirm       Подтверждать операции без запроса\n\
           -d, --verbose                Подробный вывод процесса\n\
           -f, --files                  Показать список файлов в info\n\
           -q, --quiet                  Минимальный вывод сообщений\n\
           --reinstall                  Переустановить уже установленные пакеты\n\
           --dry-run                    Симуляция обновления без записи на диск\n\
           --ignore-critical-unsafe     Игнорировать защитный барьер критических обновлений",
        "Usage: wpm [OPTIONS]... [COMMAND] [ARGS]...\n\
         Unified package manager for Wunix OS (system, user .wp packages and metapackages).\n\n\
         Commands and short flags (pacman style):\n\
           -S, install <pkg>...         Install one or more packages\n\
           reinstall <pkg>...           Force reinstall packages\n\
           -Syu                         Synchronize and upgrade all packages\n\
           -Sy, update                  Synchronize system and user repositories\n\
           -Su, upgrade                 Upgrade installed packages and kernel\n\
           -Ss, search <query>          Search packages across repositories\n\
           -Si, info <pkg>              Show package details\n\
           -Sc, clean                   Clean /var/cache/wpm\n\
           -R, remove <pkg>...          Remove user packages\n\
           -Q, list                     List installed packages\n\
           -Qu, check, updates          Check available updates\n\
           -Qi <pkg>                    Show installed package details\n\
           -U <file.wp>...              Install from local .wp file\n\
           repo [list|add|rm|ping]      Manage user repositories\n\
           audit [-v]                   Audit kernel and system file integrity\n\
           kernel [list|rollback|clean] Manage kernels in /boot\n\
           rollback                     Rollback to fallback kernel\n\n\
         Options:\n\
           -v, -V, --version            Show version information\n\
           -h, --help                   Show this help message\n\
           -c, --critical               Target critical system packages only\n\
           -y, --yes, --noconfirm       Do not prompt for confirmation\n\
           -d, --verbose                Verbose output\n\
           -f, --files                  Show file list in info\n\
           -q, --quiet                  Quiet mode\n\
           --reinstall                  Reinstall already installed packages\n\
           --dry-run                    Simulate upgrade without writing to disk\n\
           --ignore-critical-unsafe     Override critical update safety check"
    ));
}

pub fn print_access_denied(operation: &str, uid: u32) {
    println!("{}{}wpm: ошибка доступа:{} для операции '{}' требуются права администратора (root).", COLOR_BOLD, COLOR_RED, COLOR_RESET, operation);
    println!("  Текущий uid: {}", uid);
    println!("  Запустите команду через sudo: sudo wpm {} ...", operation);
}

pub fn print_wsp_blocked() {
    println!("{}{}wpm: внимание:{} обнаружены критические обновления базовой системы.", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET);
    println!("  Установка прикладного ПО временно заблокирована для сохранения стабильности ядра.");
    println!("  1. Обновите базовую систему: sudo wpm upgrade --critical");
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
