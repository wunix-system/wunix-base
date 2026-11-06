//! Модуль интеграции безопасности с системным менеджером WSP.
//!
//! Блокирует установку стороннего прикладного софта, если в системе
//! обнаружены не примененные критические обновления базы (ядро, libc, init).

use libwunix::{close, open, println, O_RDONLY};
use crate::display::{COLOR_BOLD, COLOR_GREEN, COLOR_RED, COLOR_RESET};

pub const WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

/// Проверяет, безопасна ли установка нового прикладного ПО
pub fn ensure_wsp_system_safe(ignore_unsafe: bool) -> Result<(), ()> {
    let fd = open(WSP_ALERT_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        // Базовая система в актуальном состоянии
        return Ok(());
    }
    close(fd as u64);

    if ignore_unsafe {
        println!("{}[!] ВНИМАНИЕ: Проверка безопасности WSP проигнорирована флагом --ignore-critical-unsafe!{}", COLOR_RED, COLOR_RESET);
        return Ok(());
    }

    println!("{}[!] ВНИМАНИЕ: Обнаружены не примененные критические обновления базы!{}", COLOR_RED, COLOR_RESET);
    println!("Системное ядро или базовые библиотеки устарели.");
    println!("Установка прикладных программ временно заблокирована для защиты от сбоев.");
    println!("\nЧто необходимо сделать:");
    println!("  1. Выполните: {}{}wsp upgrade --critical{}", COLOR_BOLD, COLOR_GREEN, COLOR_RESET);
    println!("  2. После перезагрузки повторите команду установки wpm.");
    println!("\n(Для принудительного игнорирования используйте флаг --ignore-critical-unsafe)");

    Err(())
}
