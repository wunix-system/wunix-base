//! Модуль чтения конфигурации демона wspd (`/etc/wsp/daemon.conf`).

extern crate alloc;
use alloc::string::{String, ToString};
use libwunix::{close, mkdir, open, read, write, O_CREAT, O_RDONLY, O_WRONLY};

pub const DAEMON_CONF_PATH: &[u8] = b"/etc/wsp/daemon.conf\0";

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub enabled: bool,
    pub check_interval: u32,
    pub check_on_boot: bool,
    pub block_on_critical: bool,
    pub log_file: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: 3600, // 1 час по умолчанию
            check_on_boot: true,
            block_on_critical: true,
            log_file: "/var/log/services/wspd.log".to_string(),
        }
    }
}

pub fn load_daemon_config() -> DaemonConfig {
    ensure_conf_dirs();
    let fd = open(DAEMON_CONF_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        let def = DaemonConfig::default();
        save_default_config(&def);
        return def;
    }

    let mut buf = [0u8; 2048];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n <= 0 {
        return DaemonConfig::default();
    }

    let text = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
    parse_daemon_config(text)
}

fn ensure_conf_dirs() {
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/wsp\0", 0o755);
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/log\0", 0o755);
    let _ = mkdir(b"/var/log/services\0", 0o755);
    let _ = mkdir(b"/run\0", 0o755);
    let _ = mkdir(b"/run/wsp\0", 0o755);
}

fn save_default_config(_conf: &DaemonConfig) {
    let fd = open(DAEMON_CONF_PATH, (O_CREAT | O_WRONLY) as u64, 0o644);
    if fd >= 0 {
        let content = b"# \xd0\x9a\xd0\xbe\xd0\xbd\xd1\x84\xd0\xb8\xd0\xb3\xd1\x83\xd1\x80\xd0\xb0\xd1\x86\xd0\xb8\xd1\x8f \xd1\x81\xd0\xb8\xd1\x81\xd1\x82\xd0\xb5\xd0\xbc\xd0\xbd\xd0\xbe\xd0\xb3\xd0\xbe \xd0\xb4\xd0\xb5\xd0\xbc\xd0\xbe\xd0\xbd\xd0\xb0 WSP (wspd)\n\
[daemon]\n\
enabled = true\n\
check_interval = 3600\n\
check_on_boot = true\n\
block_on_critical = true\n\
log_file = /var/log/services/wspd.log\n";
        let _ = write(fd as u64, content);
        close(fd as u64);
    }
}

fn parse_daemon_config(text: &str) -> DaemonConfig {
    let mut conf = DaemonConfig::default();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with('[') {
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            let k = key.trim();
            let v = val.trim().trim_matches('"').trim_matches('\'');
            match k {
                "enabled" => conf.enabled = v == "true" || v == "1" || v == "yes",
                "check_interval" => {
                    if let Ok(num) = v.parse::<u32>() {
                        conf.check_interval = num.max(10);
                    }
                }
                "check_on_boot" => conf.check_on_boot = v == "true" || v == "1" || v == "yes",
                "block_on_critical" => conf.block_on_critical = v == "true" || v == "1" || v == "yes",
                "log_file" => conf.log_file = v.to_string(),
                _ => {}
            }
        }
    }

    conf
}
