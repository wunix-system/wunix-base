//! Модуль интеграции безопасности с системным менеджером WSP.
//!
//! Блокирует установку стороннего прикладного софта, если в системе
//! обнаружены не примененные критические обновления базы (ядро, libc, init).

use libwunix::{close, open, println, t, O_RDONLY};
use crate::display::{print_error, print_warning};

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
        print_warning(t(
            "проверка безопасности WSP проигнорирована флагом --ignore-critical-unsafe!",
            "WSP security check bypassed via --ignore-critical-unsafe!",
        ));
        return Ok(());
    }

    print_error(t(
        "обнаружены обязательные обновления базовой системы WSP!",
        "mandatory WSP base system updates are available!",
    ));
    println!("{}", t(
        "wpm: установка пользовательских программ заблокирована для сохранения стабильности ядра.",
        "wpm: package installation is blocked to ensure kernel stability.",
    ));
    println!();
    println!("  1. {}", t("Обновите базовую систему: wsp upgrade --critical", "Update the base system first: wsp upgrade --critical"));
    println!("  2. {}", t("После перезагрузки повторите установку пакета wpm.", "Reboot and retry installing the application."));
    println!();
    println!("{}", t("(для принудительной установки укажите опцию --ignore-critical-unsafe)", "(to override check and force install, use --ignore-critical-unsafe)"));

    Err(())
}
