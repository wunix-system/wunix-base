use libwunix::{
    chdir, execve, exit, setgid, setuid, t, write, PasswdEntry, STDERR_FILENO,
};

/// Инициализирует окружение пользователя, переключает права доступа и запускает командную оболочку.
pub fn spawn_user_session(user_entry: &PasswdEntry, _preserve_env: bool) -> ! {
    // Порядок переключения критичен: сначала меняем GID, затем UID.
    // Если сначала вызвать setuid с переходом на непривилегированного пользователя,
    // ядро отзовет привилегии суперпользователя и следующий вызов setgid завершится ошибкой.
    let _ = setgid(user_entry.gid);
    let _ = setuid(user_entry.uid);

    // Переходим в домашний каталог пользователя. При его отсутствии отступаем в корень /.
    let home = user_entry.home();
    let mut null_home = [0u8; 128];
    let h_bytes = home.as_bytes();
    let chdir_ok = if h_bytes.len() < null_home.len() - 1 {
        null_home[..h_bytes.len()].copy_from_slice(h_bytes);
        null_home[h_bytes.len()] = 0;
        chdir(&null_home[..h_bytes.len() + 1]) == 0
    } else {
        false
    };

    if !chdir_ok {
        let _ = chdir(b"/\0");
    }

    let shell = user_entry.shell();
    let shell_path = if shell.is_empty() { "/bin/wsh" } else { shell };

    let mut null_shell = [0u8; 128];
    let s_bytes = shell_path.as_bytes();
    let s_len = s_bytes.len().min(null_shell.len() - 2);
    null_shell[..s_len].copy_from_slice(&s_bytes[..s_len]);
    null_shell[s_len] = 0;

    let argv_ptrs = [null_shell.as_ptr(), core::ptr::null()];

    // Подготавливаем базовые переменные окружения сессии
    let username = user_entry.username();
    let u_bytes = username.as_bytes();

    let mut env_user = [0u8; 64];
    copy_env_var(&mut env_user, b"USER=", u_bytes);

    let mut env_logname = [0u8; 64];
    copy_env_var(&mut env_logname, b"LOGNAME=", u_bytes);

    let mut env_home = [0u8; 128];
    copy_env_var(&mut env_home, b"HOME=", h_bytes);

    let mut env_shell = [0u8; 128];
    copy_env_var(&mut env_shell, b"SHELL=", &s_bytes[..s_len]);

    let env_path = b"PATH=/bin:/usr/bin\0";
    let env_term = b"TERM=linux\0";

    let envp_ptrs = [
        env_user.as_ptr(),
        env_logname.as_ptr(),
        env_home.as_ptr(),
        env_shell.as_ptr(),
        env_path.as_ptr(),
        env_term.as_ptr(),
        core::ptr::null(),
    ];

    let ret = execve(
        &null_shell[..s_len + 1],
        argv_ptrs.as_ptr() as u64,
        envp_ptrs.as_ptr() as u64,
    );

    // Если основной шелл не найден, пробуем резервный /bin/wsh
    if ret < 0 && shell_path != "/bin/wsh" {
        let fallback = b"/bin/wsh\0";
        let fallback_argv = [fallback.as_ptr(), core::ptr::null()];
        let _ = execve(
            fallback,
            fallback_argv.as_ptr() as u64,
            envp_ptrs.as_ptr() as u64,
        );
    }

    if ret >= 0 {
        // Успешный выход из сессии шелла
        exit(0);
    }

    let err_msg = t(
        "login: невозможно запустить командную оболочку\n",
        "login: cannot execute user shell\n",
    );
    write(STDERR_FILENO, err_msg.as_bytes());
    exit(1);
}

fn copy_env_var(dest: &mut [u8], prefix: &[u8], value: &[u8]) {
    let p_len = prefix.len();
    dest[..p_len].copy_from_slice(prefix);
    let v_max = dest.len().saturating_sub(p_len + 1);
    let v_len = value.len().min(v_max);
    dest[p_len..p_len + v_len].copy_from_slice(&value[..v_len]);
    dest[p_len + v_len] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_env_var() {
        let mut buf = [0u8; 32];
        copy_env_var(&mut buf, b"USER=", b"alice");
        assert_eq!(&buf[..11], b"USER=alice\0");
    }
}
