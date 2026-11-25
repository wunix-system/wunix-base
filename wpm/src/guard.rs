//! Модуль проверки безопасности перед установкой прикладных программ WPM.

extern crate alloc;
use alloc::vec::Vec;
use libwunix::{close, open, read, t, O_RDONLY};
use libpackwunix::sources::WspConfig;

use crate::db::has_critical_alert;
use crate::display::{print_warning, print_wsp_blocked};

pub const WPM_CONF_PATH: &[u8] = b"/etc/wpm/wpm.conf\0";
pub const LEGACY_WSP_CONF_PATH: &[u8] = b"/etc/wsp/wsp.conf\0";

fn read_wpm_conf() -> WspConfig {
    let mut fd = open(WPM_CONF_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        fd = open(LEGACY_WSP_CONF_PATH, O_RDONLY as u64, 0);
    }
    if fd < 0 {
        return WspConfig::default();
    }

    let mut buf = [0u8; 4096];
    let mut text_vec = Vec::new();
    loop {
        let n = read(fd as u64, &mut buf);
        if n <= 0 {
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
pub fn ensure_system_safe_for_user_apps(ignore_unsafe: bool) -> Result<(), ()> {
    let cfg = read_wpm_conf();

    if !cfg.block_user_packages_on_critical || !has_critical_alert() {
        return Ok(());
    }

    if ignore_unsafe {
        print_warning(t(
            "проверка безопасности критических обновлений проигнорирована (--ignore-critical-unsafe).",
            "critical system update safety check bypassed (--ignore-critical-unsafe).",
        ));
        return Ok(());
    }

    print_wsp_blocked();
    Err(())
}
