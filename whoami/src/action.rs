use libwunix::{geteuid, get_passwd_by_uid, write, STDOUT_FILENO};

pub fn run_whoami() {
    let euid = geteuid();
    if let Ok(Some(entry)) = get_passwd_by_uid(euid) {
        write(STDOUT_FILENO, entry.username().as_bytes());
        write(STDOUT_FILENO, b"\n");
    } else {
        // Fallback: если пользователь не найден в базе, выводим числовой EUID
        let mut num_buf = [0u8; 16];
        let s = libwunix::fmt::utoa(euid as u64, &mut num_buf);
        write(STDOUT_FILENO, s.as_bytes());
        write(STDOUT_FILENO, b"\n");
    }
}
