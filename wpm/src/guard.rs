//! Модуль интеграции безопасности WPM с системной конфигурацией WSP.
//!
//! Блокирует установку прикладных программ, если в /etc/wsp/wsp.conf включена
//! защита `block_user_packages_on_critical` и обнаружен активный флаг критического
//! обновления ядра или системных библиотек (/run/wsp/critical_alert.json).

extern crate alloc;
use alloc::vec::Vec;
use libwunix::{close, open, read, t, O_RDONLY};
use libpackwunix::sources::WspConfig;

use crate::display::{print_warning, print_wsp_blocked};

pub const WSP_CONF_PATH: &[u8] = b"/etc/wsp/wsp.conf\0";
pub const WSP_ALERT_PATH: &[u8] = b"/run/wsp/critical_alert.json\0";

/// Считывает текстовый файл конфигурации WSP
fn read_wsp_conf() -> WspConfig {
    let fd = open(WSP_CONF_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        return WspConfig::default();
    }

    let mut buf = [0u8; 4096];
    let mut text_vec = Vec::new();
    loop {
        let n = read(fd as u64, &mut buf);
        if n == 0 {
            break;
        }
        text_vec.extend_from_slice(&buf[..n as usize]);
    }
    close(fd as u64);

    if let Ok(text) = core::str::from_utf8(&text_vec) {
        WspConfig::parse(text)
    } else {
        WspConfig::default()
    }
}

/// Проверяет, безопасна ли установка нового прикладного ПО
pub fn ensure_wsp_system_safe(ignore_unsafe: bool) -> Result<(), ()> {
    let wsp_cfg = read_wsp_conf();

    if !wsp_cfg.block_user_packages_on_critical {
        // Блокировка отключена администратором в /etc/wsp/wsp.conf
        return Ok(());
    }

    let fd = open(WSP_ALERT_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        // Системных критических предупреждений нет
        return Ok(());
    }
    close(fd as u64);

    if ignore_unsafe {
        print_warning(t(
            "проверка безопасности WSP проигнорирована опцией --ignore-critical-unsafe!",
            "WSP security check bypassed via --ignore-critical-unsafe!",
        ));
        return Ok(());
    }

    // Выводим карточку блокировки WSP
    print_wsp_blocked();
    Err(())
}
