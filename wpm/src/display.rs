//! Модуль отображения и фирменного интерфейса WPM (Wunix Package Manager).
//!
//! Реализует самобытный, структурированный и дружелюбный визуальный стиль Wunix,
//! не похожий на утилиты других ОС. Поддерживает двуязычную локализацию t().

extern crate alloc;
use alloc::format;
use alloc::string::String;
use libwunix::{println, t};

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_DIM: &str = "\x1b[2m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_CYAN: &str = "\x1b[36m";
pub const COLOR_WHITE: &str = "\x1b[37m";

/// Форматирует размер в байтах в человекочитаемый вид
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

/// Фирменная карточка версии WPM
pub fn print_version() {
    println!();
    println!("  {}{}◆ WPM ── Менеджер приложений Wunix (v1.1.0){}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("    {:<18} {}", t("Платформа:", "Platform:"), "Wunix OS (x86_64 / Long Mode)");
    println!("    {:<18} {}", t("Назначение:", "Scope:"), t("Пользовательские программы и утилиты разработчика", "User applications and developer tools"));
    println!("    {:<18} {}", t("Формат пакетов:", "Package format:"), "Wunix Package (.wp)");
    println!("    {:<18} {}", t("Источники:", "Sources:"), "/etc/wpm/sources.conf");
    println!("    {:<18} {}", t("Реестр программ:", "Registry db:"), "/var/lib/wpm/installed.json");
    println!("    {:<18} {}", t("Каталог кэша:", "Cache dir:"), "/var/cache/wpm/");
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t("Для просмотра всех доступных команд используйте: wpm --help", "For available commands and flags use: wpm --help"));
    println!();
}

/// Фирменная справка WPM
pub fn print_help() {
    println!();
    println!("  {}{}◆ WPM ── Менеджер приложений Wunix{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t(
        "Управление прикладными программами, компиляторами и пакетами (.wp)",
        "Management of user applications, developer tools, and .wp packages"
    ));
    println!();
    println!("  {}{}КОМАНДЫ:{}", COLOR_BOLD, COLOR_WHITE, COLOR_RESET);
    println!("    {:<18} {}", "search <запрос>", t("Поиск программ в доступных репозиториях", "Search available packages"));
    println!("    {:<18} {}", "install <программа>", t("Установка прикладных пакетов (.wp)", "Install user applications"));
    println!("    {:<18} {}", "remove <программа>", t("Удаление установленных программ", "Remove installed applications"));
    println!("    {:<18} {}", "upgrade", t("Обновление установленных программ до свежих версий", "Upgrade installed applications"));
    println!("    {:<18} {}", "list", t("Список всех установленных программ", "List installed applications"));
    println!("    {:<18} {}", "info <программа>", t("Подробная карточка программы и список файлов", "Detailed package information"));
    println!("    {:<18} {}", "repo [list|add|rm]", t("Управление источниками репозиториев WPM", "Manage repository sources"));
    println!("    {:<18} {}", "update", t("Синхронизация индексов репозиториев", "Synchronize repository index"));
    println!("    {:<18} {}", "clean", t("Очистка локального кэша пакетов /var/cache/wpm", "Clean downloaded packages cache"));
    println!();
    println!("  {}{}ПАРАМЕТРЫ:{}", COLOR_BOLD, COLOR_WHITE, COLOR_RESET);
    println!("    {:<18} {}", "-v, --version", t("Показать карточку версии WPM", "Display WPM version card"));
    println!("    {:<18} {}", "-h, --help", t("Показать эту справку", "Display this help message"));
    println!("    {:<18} {}", "-y, --yes", t("Не запрашивать подтверждение при установке", "Assume yes for prompts"));
    println!("    {:<18} {}", "-d, --verbose", t("Подробный протокол распаковки файлов", "Verbose extraction output"));
    println!("    {:<18} {}", "--ignore-critical-unsafe", t("Игнорировать блокировку безопасности WSP", "Override WSP security gate"));
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}: {}", t("Примечание", "Note"), t("Для установки и удаления программ требуются права 'sudo'", "Installation and removal require 'sudo' privileges"));
    println!();
}

/// Карточка ошибки отказа в доступе (без sudo)
pub fn print_access_denied(operation: &str, uid: u32) {
    println!();
    println!("  {}{}◆ WPM ── Ограничение доступа{}", COLOR_BOLD, COLOR_RED, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t(
        "Для изменения системных каталогов (/bin, /usr) требуются права администратора.",
        "Administrative privileges are required to modify system paths (/bin, /usr)."
    ));
    println!("  {}: {} (uid={})", t("Текущий пользователь", "Current caller"), if uid == 0 { "root" } else { "пользователь" }, uid);
    println!();
    println!("  {}:", t("Пожалуйста, запустите команду через sudo", "Please run this command with sudo"));
    println!("    {}sudo wpm {} ...{}", COLOR_BOLD, operation, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!();
}

/// Карточка блокировки при наличии критических обновлений WSP
pub fn print_wsp_blocked() {
    println!();
    println!("  {}{}◆ WPM ── Защитный барьер безопасности WSP{}", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t(
        "В базовой системе обнаружены обязательные обновления ядра или системных библиотек.",
        "Critical base system updates are pending in the system repository."
    ));
    println!("  {}", t(
        "Установка сторонних программ временно приостановлена политикой безопасности WSP.",
        "Installing user applications is paused to prevent kernel/library inconsistency."
    ));
    println!();
    println!("  {}:", t("Рекомендуемый порядок действий", "Recommended actions"));
    println!("    1. {}{}{}", COLOR_BOLD, t("sudo wsp upgrade --critical", "sudo wsp upgrade --critical"), COLOR_RESET);
    println!("    2. {}{}{}", COLOR_BOLD, t("reboot", "reboot"), COLOR_RESET);
    println!("    3. {}", t("Повторите установку нужной программы в WPM", "Retry installing your package in WPM"));
    println!();
    println!("  {}", t("(для принудительной установки укажите флаг: --ignore-critical-unsafe)", "(to override check use: --ignore-critical-unsafe)"));
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!();
}

/// Вывод шага операции
pub fn print_step(current: usize, total: usize, desc: &str, status_ok: bool) {
    let status_str = if status_ok {
        format!("{}[ OK ]{}", COLOR_GREEN, COLOR_RESET)
    } else {
        format!("{}[FAIL]{}", COLOR_RED, COLOR_RESET)
    };
    println!("  [{}/{}] {:<45} {}", current, total, desc, status_str);
}

pub fn print_info(msg: &str) {
    println!("  {}•{} {}", COLOR_CYAN, COLOR_RESET, msg);
}

pub fn print_warning(msg: &str) {
    println!("  {}{}Внимание:{} {}", COLOR_BOLD, COLOR_YELLOW, COLOR_RESET, msg);
}

pub fn print_error(msg: &str) {
    println!("  {}{}Ошибка:{} {}", COLOR_BOLD, COLOR_RED, COLOR_RESET, msg);
}
