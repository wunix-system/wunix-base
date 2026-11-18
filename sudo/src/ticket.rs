use libwunix::{close, mkdir, open, read, unlink, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};

pub const TICKET_MAGIC: [u8; 4] = *b"WTS1";
pub const TICKET_VERSION: u32 = 1;

/// Таймаут кэширования пароля в секундах (15 минут — классический стандарт POSIX/Linux sudo)
pub const TICKET_TIMEOUT_SECS: i64 = 15 * 60;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct SudoTicket {
    pub magic: [u8; 4],
    pub version: u32,
    pub boot_epoch: u64,
    pub monotonic_sec: i64,
}

/// Формирует путь к файлу билета в `/var/run/sudo/ts/<username>`.
pub fn get_ticket_path<'a>(username: &str, out: &'a mut [u8]) -> Option<&'a [u8]> {
    if username.is_empty() || username.len() > 32 {
        return None;
    }
    for b in username.bytes() {
        if !(b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.') {
            return None;
        }
    }
    if username == "." || username == ".." {
        return None;
    }

    let prefix = b"/var/run/sudo/ts/";
    let total_len = prefix.len() + username.len() + 1;
    if total_len > out.len() {
        return None;
    }

    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..prefix.len() + username.len()].copy_from_slice(username.as_bytes());
    out[prefix.len() + username.len()] = 0;

    Some(&out[..total_len])
}

/// Проверяет валидность билета sudo без обращения к диску (чистая функция).
pub fn is_ticket_valid(ticket: &SudoTicket, now_mono: i64, now_boot: u64, timeout_secs: i64) -> bool {
    if ticket.magic != TICKET_MAGIC || ticket.version != TICKET_VERSION {
        return false;
    }
    if now_boot != 0 && ticket.boot_epoch != 0 && ticket.boot_epoch != now_boot {
        return false;
    }
    if now_mono < ticket.monotonic_sec {
        return false;
    }
    if now_mono - ticket.monotonic_sec > timeout_secs {
        return false;
    }
    true
}

fn ensure_ticket_dir() {
    let _ = mkdir(b"/var\0", 0o755);
    let _ = mkdir(b"/var/run\0", 0o755);
    let _ = mkdir(b"/var/run/sudo\0", 0o700);
    let _ = mkdir(b"/var/run/sudo/ts\0", 0o700);
}

pub fn read_ticket(username: &str) -> Option<SudoTicket> {
    let mut path_buf = [0u8; 96];
    let path = get_ticket_path(username, &mut path_buf)?;

    let fd = open(path, O_RDONLY as u64, 0);
    if fd < 0 {
        return None;
    }

    let mut ticket_bytes = [0u8; core::mem::size_of::<SudoTicket>()];
    let n = read(fd as u64, &mut ticket_bytes);
    let _ = close(fd as u64);

    if n as usize != core::mem::size_of::<SudoTicket>() {
        return None;
    }

    let ticket: SudoTicket = unsafe {
        core::ptr::read_unaligned(ticket_bytes.as_ptr() as *const SudoTicket)
    };
    Some(ticket)
}

pub fn write_ticket(username: &str, ticket: &SudoTicket) -> bool {
    ensure_ticket_dir();

    let mut path_buf = [0u8; 96];
    let path = match get_ticket_path(username, &mut path_buf) {
        Some(p) => p,
        None => return false,
    };

    let flags = O_WRONLY | O_CREAT | O_TRUNC;
    let fd = open(path, flags as u64, 0o600);
    if fd < 0 {
        return false;
    }

    let ticket_bytes = unsafe {
        core::slice::from_raw_parts(
            ticket as *const SudoTicket as *const u8,
            core::mem::size_of::<SudoTicket>(),
        )
    };
    let written = write(fd as u64, ticket_bytes);
    let _ = close(fd as u64);

    written as usize == core::mem::size_of::<SudoTicket>()
}

pub fn remove_ticket(username: &str) -> bool {
    let mut path_buf = [0u8; 96];
    let path = match get_ticket_path(username, &mut path_buf) {
        Some(p) => p,
        None => return false,
    };
    unlink(path) == 0
}

/// Проверяет наличие сохранённого и ещё не истекшего билета для пользователя.
pub fn check_cached_auth(username: &str) -> bool {
    let ticket = match read_ticket(username) {
        Some(t) => t,
        None => return false,
    };
    let now_mono = libwunix::time::clock_monotonic().tv_sec;
    let now_boot = libwunix::syslib::boot_timestamp();
    is_ticket_valid(&ticket, now_mono, now_boot, TICKET_TIMEOUT_SECS)
}

/// Создаёт или обновляет отметку времени билета при успешной аутентификации.
pub fn touch_cached_auth(username: &str) -> bool {
    let ticket = SudoTicket {
        magic: TICKET_MAGIC,
        version: TICKET_VERSION,
        boot_epoch: libwunix::syslib::boot_timestamp(),
        monotonic_sec: libwunix::time::clock_monotonic().tv_sec,
    };
    write_ticket(username, &ticket)
}

/// Сбрасывает сохранённый билет (для `sudo -k`).
pub fn remove_cached_auth(username: &str) -> bool {
    remove_ticket(username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_valid_within_timeout() {
        let ticket = SudoTicket {
            magic: TICKET_MAGIC,
            version: TICKET_VERSION,
            boot_epoch: 1000,
            monotonic_sec: 200,
        };
        assert!(is_ticket_valid(&ticket, 300, 1000, 900));
    }

    #[test]
    fn test_ticket_expired_after_timeout() {
        let ticket = SudoTicket {
            magic: TICKET_MAGIC,
            version: TICKET_VERSION,
            boot_epoch: 1000,
            monotonic_sec: 100,
        };
        assert!(!is_ticket_valid(&ticket, 1001, 1000, 900));
    }

    #[test]
    fn test_ticket_invalid_on_different_boot() {
        let ticket = SudoTicket {
            magic: TICKET_MAGIC,
            version: TICKET_VERSION,
            boot_epoch: 1000,
            monotonic_sec: 200,
        };
        assert!(!is_ticket_valid(&ticket, 300, 2000, 900));
    }

    #[test]
    fn test_ticket_invalid_on_time_inversion() {
        let ticket = SudoTicket {
            magic: TICKET_MAGIC,
            version: TICKET_VERSION,
            boot_epoch: 1000,
            monotonic_sec: 500,
        };
        assert!(!is_ticket_valid(&ticket, 400, 1000, 900));
    }

    #[test]
    fn test_ticket_invalid_magic_or_version() {
        let bad_magic = SudoTicket {
            magic: *b"BAD!",
            version: TICKET_VERSION,
            boot_epoch: 1000,
            monotonic_sec: 200,
        };
        assert!(!is_ticket_valid(&bad_magic, 300, 1000, 900));

        let bad_ver = SudoTicket {
            magic: TICKET_MAGIC,
            version: 99,
            boot_epoch: 1000,
            monotonic_sec: 200,
        };
        assert!(!is_ticket_valid(&bad_ver, 300, 1000, 900));
    }

    #[test]
    fn test_get_ticket_path_valid() {
        let mut buf = [0u8; 64];
        let path = get_ticket_path("alice", &mut buf).unwrap();
        assert_eq!(path, b"/var/run/sudo/ts/alice\0");
    }

    #[test]
    fn test_get_ticket_path_rejects_malformed_username() {
        let mut buf = [0u8; 64];
        assert!(get_ticket_path("", &mut buf).is_none());
        assert!(get_ticket_path("..", &mut buf).is_none());
        assert!(get_ticket_path(".", &mut buf).is_none());
        assert!(get_ticket_path("user/name", &mut buf).is_none());
        assert!(get_ticket_path("user\\name", &mut buf).is_none());
        assert!(get_ticket_path("user name", &mut buf).is_none());
    }
}
