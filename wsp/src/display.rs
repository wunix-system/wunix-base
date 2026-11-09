//! Модуль отображения и фирменного интерфейса WSP (Wunix System Package).
//!
//! Реализует строгий, надежный и структурированный визуальный стиль Wunix
//! для управления ядром, системными библиотеками и загрузчиком.

extern crate alloc;
use alloc::format;
use libwunix::{println, t};
use libpackwunix::sources::get_wsp_system_repo;

pub const COLOR_RESET: &str = "\x1b[0m";
pub const COLOR_BOLD: &str = "\x1b[1m";
pub const COLOR_DIM: &str = "\x1b[2m";
pub const COLOR_RED: &str = "\x1b[31m";
pub const COLOR_GREEN: &str = "\x1b[32m";
pub const COLOR_YELLOW: &str = "\x1b[33m";
pub const COLOR_CYAN: &str = "\x1b[36m";
pub const COLOR_WHITE: &str = "\x1b[37m";

/// Фирменная карточка версии WSP
pub fn print_version() {
    println!();
    println!("  {}{}◆ WSP ── Системный менеджер Wunix (v1.1.0){}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("    {:<18} {}", t("Платформа:", "Platform:"), "Wunix OS (x86_64 / Long Mode)");
    println!("    {:<18} {}", t("Назначение:", "Scope:"), t("Ядро, системные библиотеки Ring 3 и загрузчик", "Monolithic kernel, base system, and bootloader"));
    println!("    {:<18} {}", t("Формат пакетов:", "Package format:"), "Wunix Package (.wp)");
    println!("    {:<18} {} {}", t("Репозиторий:", "Repository:"), get_wsp_system_repo(), t("[встроенный, защищен]", "[builtin, immutable]"));
    println!("    {:<18} {}", t("Конфигурация:", "Configuration:"), "/etc/wsp/wsp.conf");
    println!("    {:<18} {}", t("Реестр системы:", "Registry db:"), "/var/lib/wsp/installed.json");
    println!("    {:<18} {}", t("Загрузчик ядра:", "Bootloader:"), "Limine (boot():/boot/wunix-kernel)");
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t("Для просмотра всех доступных команд используйте: wsp --help", "For available commands and flags use: wsp --help"));
    println!();
}

/// Фирменная справка WSP
pub fn print_help() {
    println!();
    println!("  {}{}◆ WSP ── Системный менеджер Wunix{}", COLOR_BOLD, COLOR_CYAN, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t(
        "Диспетчер системных пакетов, ротации ядра и безопасности базового образа (.wp)",
        "System package manager, kernel rotation, and base image security (.wp)"
    ));
    println!();
    println!("  {}{}КОМАНДЫ:{}", COLOR_BOLD, COLOR_WHITE, COLOR_RESET);
    println!("    {:<18} {}", "audit", t("Полный аудит ядра, контрольных сумм файлов и безопасности", "Audit kernel, system files, and security"));
    println!("    {:<18} {}", "check", t("Быстрая проверка наличия системных обновлений ядра", "Check for available kernel/base updates"));
    println!("    {:<18} {}", "upgrade", t("Применение обновлений ядра с созданием точки отката", "Upgrade kernel and base system with rollback"));
    println!("    {:<18} {}", "list", t("Список установленных системных компонентов ядра", "List installed base components"));
    println!("    {:<18} {}", "info <пакет>", t("Подробная информация о системном компоненте", "Detailed component information"));
    println!("    {:<18} {}", "install <пакет.wp>", t("Установка системного компонента из файла .wp", "Install a system package from .wp"));
    println!("    {:<18} {}", "kernel [list|rollback]", t("Управление установленными ядрами в загрузчике", "Manage kernel entries in bootloader"));
    println!("    {:<18} {}", "rollback", t("Откат к предыдущей версии ядра (/boot/wunix-kernel.old)", "Rollback to previous kernel"));
    println!("    {:<18} {}", "update", t("Синхронизация индекса системного репозитория", "Synchronize system repository index"));
    println!();
    println!("  {}{}ПАРАМЕТРЫ:{}", COLOR_BOLD, COLOR_WHITE, COLOR_RESET);
    println!("    {:<18} {}", "-v, --version", t("Показать карточку версии WSP", "Display WSP version card"));
    println!("    {:<18} {}", "-h, --help", t("Показать эту справку", "Display this help message"));
    println!("    {:<18} {}", "-c, --critical", t("Обрабатывать только критические компоненты базы", "Target critical packages only"));
    println!("    {:<18} {}", "-y, --yes", t("Не запрашивать подтверждение при обновлении", "Assume yes for prompts"));
    println!("    {:<18} {}", "-d, --verbose", t("Подробный протокол проверки и распаковки файлов", "Verbose execution details"));
    println!("    {:<18} {}", "--dry-run", t("Режим симуляции без изменения файлов ядра", "Simulation mode without disk writes"));
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}: {}", t("Безопасность", "Security"), t("Адрес репозитория WSP встроен в код и защищен от подмены", "WSP repository URL is built-in and protected against tampering"));
    println!();
}

/// Карточка ошибки отказа в доступе (без sudo)
pub fn print_access_denied(operation: &str, uid: u32) {
    println!();
    println!("  {}{}◆ WSP ── Ограничение доступа к ядру{}", COLOR_BOLD, COLOR_RED, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!("  {}", t(
        "Управление ядром и базовой системой Wunix разрешено только суперпользователю (root).",
        "Modifying kernel and base system requires superuser (root) privileges."
    ));
    println!("  {}: {} (uid={})", t("Текущий пользователь", "Current caller"), if uid == 0 { "root" } else { "пользователь" }, uid);
    println!();
    println!("  {}:", t("Пожалуйста, запустите команду через sudo", "Please run this command with sudo"));
    println!("    {}sudo wsp {} ...{}", COLOR_BOLD, operation, COLOR_RESET);
    println!("  {}────────────────────────────────────────────────────────────{}", COLOR_DIM, COLOR_RESET);
    println!();
}

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

pub fn print_error(msg: &str) {
    println!("  {}{}Ошибка:{} {}", COLOR_BOLD, COLOR_RED, COLOR_RESET, msg);
}
