use libwunix::{
    close, delete_user_from_all, get_passwd_by_name, getdents64, geteuid, open, rmdir, t, unlink,
    write, O_RDONLY, STDERR_FILENO,
};

fn remove_dir_recursive(path: &str) {
    let mut null_path = [0u8; 1024];
    let bytes = path.as_bytes();
    if bytes.len() >= 1023 {
        return;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let fd = open(&null_path[..bytes.len() + 1], O_RDONLY as u64, 0);
    if fd >= 0 {
        let mut buf = [0u8; 1024];
        loop {
            let n = getdents64(fd as u64, &mut buf);
            if n <= 0 {
                break;
            }
            let mut pos = 0;
            while pos + 19 <= n as usize {
                let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
                if reclen == 0 || pos + reclen > n as usize {
                    break;
                }
                let dtype = buf[pos + 18];
                let name_start = pos + 19;
                let mut name_end = name_start;
                while name_end < pos + reclen && buf[name_end] != 0 {
                    name_end += 1;
                }
                if let Ok(name) = core::str::from_utf8(&buf[name_start..name_end]) {
                    if name != "." && name != ".." {
                        let mut sub_buf = [0u8; 1024];
                        let mut s_len = 0;
                        sub_buf[..bytes.len()].copy_from_slice(bytes);
                        s_len += bytes.len();
                        if !path.ends_with('/') {
                            sub_buf[s_len] = b'/';
                            s_len += 1;
                        }
                        sub_buf[s_len..s_len + name.len()].copy_from_slice(name.as_bytes());
                        s_len += name.len();
                        sub_buf[s_len] = 0;

                        if let Ok(sub_path) = core::str::from_utf8(&sub_buf[..s_len]) {
                            if dtype == 4 {
                                remove_dir_recursive(sub_path);
                            } else {
                                let _ = unlink(&sub_buf[..s_len + 1]);
                            }
                        }
                    }
                }
                pos += reclen;
            }
        }
        let _ = close(fd as u64);
    }

    let _ = rmdir(&null_path[..bytes.len() + 1]);
}

pub fn run_userdel(username: &str, remove_home: bool) -> bool {
    // 1. Проверка прав root
    if geteuid() != 0 {
        let err = t("userdel: только root может удалять пользователей\n", "userdel: only root may remove a user\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // 2. Получаем запись пользователя, чтобы узнать домашний каталог
    let user_entry = match get_passwd_by_name(username) {
        Ok(Some(e)) => e,
        _ => {
            let err_pre = t("userdel: пользователь '", "userdel: user '");
            let err_post = t("': не существует\n", "': does not exist\n");
            write(STDERR_FILENO, err_pre.as_bytes());
            write(STDERR_FILENO, username.as_bytes());
            write(STDERR_FILENO, err_post.as_bytes());
            return false;
        }
    };

    let home_path = user_entry.home();

    // 3. Удаление пользователя из всех баз (/etc/passwd, /etc/shadow, /etc/group)
    if let Err(_) = delete_user_from_all(username) {
        let err = t("userdel: не удалось обновить конфигурационные файлы\n", "userdel: failed to update configuration files\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // 4. Удаление домашнего каталога при флаге -r
    if remove_home && !home_path.is_empty() && home_path != "/" && home_path != "/root" {
        remove_dir_recursive(home_path);
    }

    true
}
