#![cfg_attr(test, allow(dead_code))]

use crate::auth::authenticate_user;
use crate::ticket::{check_cached_auth, remove_cached_auth, touch_cached_auth};
use libwunix::{
    execve, get_passwd_by_uid, getuid, setgid, setuid, t, user_in_group, write, STDERR_FILENO,
};

/// Сбрасывает сохранённые учетные данные пользователя (для `sudo -k`).
pub fn run_reset_timestamp() -> bool {
    let real_uid = getuid();
    if real_uid == 0 {
        return true;
    }
    if let Ok(Some(entry)) = get_passwd_by_uid(real_uid) {
        remove_cached_auth(entry.username());
    }
    true
}

/// Проверяет или обновляет сохранённый билет без запуска команды (для `sudo -v`).
pub fn run_validate() -> bool {
    let real_uid = getuid();
    if real_uid == 0 {
        return true;
    }

    let user_entry = match get_passwd_by_uid(real_uid) {
        Ok(Some(e)) => e,
        _ => {
            let err = t("sudo: текущий пользователь не найден\n", "sudo: current user not found\n");
            write(STDERR_FILENO, err.as_bytes());
            return false;
        }
    };
    let username = user_entry.username();

    if !user_in_group(username, "wheel") {
        let err_pre = t("sudo: пользователь '", "sudo: user '");
        let err_post = t("': отсутствует в файле sudoers (группа wheel)\n", "': is not in the sudoers file (wheel group)\n");
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, username.as_bytes());
        write(STDERR_FILENO, err_post.as_bytes());
        return false;
    }

    if check_cached_auth(username) {
        touch_cached_auth(username);
        return true;
    }

    if !authenticate_user(username) {
        return false;
    }

    touch_cached_auth(username);
    true
}

/// Выполняет команду от имени root с проверкой прав, кэшированием пароля и эскалацией.
pub fn run_sudo(cmd_path: &str, argv: *const *const u8, start_idx: usize, argc: usize) -> bool {
    let real_uid = getuid();

    let user_entry = match get_passwd_by_uid(real_uid) {
        Ok(Some(e)) => e,
        _ => {
            let err = t("sudo: текущий пользователь не найден\n", "sudo: current user not found\n");
            write(STDERR_FILENO, err.as_bytes());
            return false;
        }
    };

    let username = user_entry.username();

    // 1. Проверяем членство в группе wheel
    if real_uid != 0 && !user_in_group(username, "wheel") {
        let err_pre = t("sudo: пользователь '", "sudo: user '");
        let err_post = t("': отсутствует в файле sudoers (группа wheel)\n", "': is not in the sudoers file (wheel group)\n");
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, username.as_bytes());
        write(STDERR_FILENO, err_post.as_bytes());
        return false;
    }

    // 2. Аутентификация (запрос пароля только если нет действующего билета)
    if real_uid != 0 {
        if !check_cached_auth(username) {
            if !authenticate_user(username) {
                return false;
            }

            // Запоминаем успешную аутентификацию
            touch_cached_auth(username);
        } else {
            // Продлеваем текущий активный сеанс sudo
            touch_cached_auth(username);
        }
    }

    // 3. Повышение привилегий до root
    let _ = setgid(0);
    let _ = setuid(0);

    // 4. Формирование argv для запуска команды
    let mut null_cmd = [0u8; 128];
    let c_bytes = cmd_path.as_bytes();
    if c_bytes.len() >= null_cmd.len() - 1 {
        return false;
    }
    null_cmd[..c_bytes.len()].copy_from_slice(c_bytes);
    null_cmd[c_bytes.len()] = 0;

    let mut argv_storage: [*const u8; 32] = [core::ptr::null(); 32];
    let count = (argc - start_idx).min(31);
    for i in 0..count {
        argv_storage[i] = unsafe { *argv.add(start_idx + i) };
    }
    argv_storage[count] = core::ptr::null();

    let env_user = b"USER=root\0";
    let env_home = b"HOME=/root\0";
    let env_path = b"PATH=/bin:/usr/bin\0";
    let env_term = b"TERM=linux\0";

    let envp_ptrs = [
        env_user.as_ptr(),
        env_home.as_ptr(),
        env_path.as_ptr(),
        env_term.as_ptr(),
        core::ptr::null(),
    ];

    let ret = execve(
        &null_cmd[..c_bytes.len() + 1],
        argv_storage.as_ptr() as u64,
        envp_ptrs.as_ptr() as u64,
    );

    if ret < 0 {
        let err_pre = t("sudo: ошибка запуска команды '", "sudo: failed to execute command '");
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, cmd_path.as_bytes());
        write(STDERR_FILENO, b"'\n");
        return false;
    }

    true
}
