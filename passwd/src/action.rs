use crate::terminal::read_password_line;
use libwunix::{
    crypto::{generate_salt, hash_password},
    get_passwd_by_name, get_passwd_by_uid, getuid, t, update_shadow_password, verify_password,
    write, STDERR_FILENO, STDOUT_FILENO,
};

pub fn run_passwd(target_username: Option<&str>) -> bool {
    let real_uid = getuid();

    // Определяем имя целевого пользователя
    let mut username_buf = [0u8; 32];
    let username_len;

    if let Some(target) = target_username {
        if real_uid != 0 {
            let caller_entry = match get_passwd_by_uid(real_uid) {
                Ok(Some(e)) => e,
                _ => {
                    let err = t("passwd: вы не можете менять чужой пароль.\n", "passwd: you may not view or modify password for another user.\n");
                    write(STDERR_FILENO, err.as_bytes());
                    return false;
                }
            };
            if caller_entry.username() != target {
                let err = t("passwd: только root может указывать имя пользователя.\n", "passwd: only root can specify a user name.\n");
                write(STDERR_FILENO, err.as_bytes());
                return false;
            }
        }
        let b = target.as_bytes();
        let copy_len = b.len().min(32);
        username_buf[..copy_len].copy_from_slice(&b[..copy_len]);
        username_len = copy_len;
    } else {
        match get_passwd_by_uid(real_uid) {
            Ok(Some(e)) => {
                let u = e.username();
                let b = u.as_bytes();
                let copy_len = b.len().min(32);
                username_buf[..copy_len].copy_from_slice(&b[..copy_len]);
                username_len = copy_len;
            }
            _ => {
                let err = t("passwd: текущий пользователь не найден в /etc/passwd\n", "passwd: user not found in /etc/passwd\n");
                write(STDERR_FILENO, err.as_bytes());
                return false;
            }
        }
    }

    let username = match core::str::from_utf8(&username_buf[..username_len]) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if get_passwd_by_name(username).ok().flatten().is_none() {
        let err_pre = t("passwd: пользователь '", "passwd: user '");
        let err_post = t("': не существует\n", "': does not exist\n");
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, username.as_bytes());
        write(STDERR_FILENO, err_post.as_bytes());
        return false;
    }

    // Для обычного пользователя запрашиваем старый пароль
    if real_uid != 0 {
        let prompt_old = t("Текущий пароль: ", "Current password: ");
        write(STDOUT_FILENO, prompt_old.as_bytes());
        let mut old_buf = [0u8; 128];
        let old_pwd = read_password_line(&mut old_buf);

        if verify_password(username, old_pwd).is_err() {
            let err = t("passwd: неверный пароль\n", "passwd: Authentication token manipulation error: Incorrect password\n");
            write(STDERR_FILENO, err.as_bytes());
            return false;
        }
    }

    // Запрос нового пароля
    let prompt_new = t("Новый пароль: ", "New password: ");
    write(STDOUT_FILENO, prompt_new.as_bytes());
    let mut new_buf1 = [0u8; 128];
    let new_pwd1 = read_password_line(&mut new_buf1);

    if new_pwd1.is_empty() {
        let err = t("passwd: пустой пароль недопустим\n", "passwd: empty password is not allowed\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // Подтверждение нового пароля
    let prompt_retype = t("Повторите новый пароль: ", "Retype new password: ");
    write(STDOUT_FILENO, prompt_retype.as_bytes());
    let mut new_buf2 = [0u8; 128];
    let new_pwd2 = read_password_line(&mut new_buf2);

    if new_pwd1 != new_pwd2 {
        let mismatch = t("passwd: пароли не совпадают\n", "passwd: passwords do not match\n");
        write(STDERR_FILENO, mismatch.as_bytes());
        return false;
    }

    // Генерация новой соли и хеша пароля
    let mut salt_raw = [0u8; 8];
    let seed = crate::time::get_seed();
    generate_salt(seed, &mut salt_raw);
    let salt = match core::str::from_utf8(&salt_raw) {
        Ok(s) => s,
        Err(_) => "wunixsalt",
    };

    let mut hash_buf = [0u8; 128];
    let hash_len = hash_password(new_pwd1, salt, &mut hash_buf);
    let hash_str = match core::str::from_utf8(&hash_buf[..hash_len]) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if let Err(_) = update_shadow_password(username, hash_str) {
        let err = t("passwd: не удалось обновить /etc/shadow\n", "passwd: failed to update /etc/shadow\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    let success = t("passwd: пароль успешно обновлен\n", "passwd: password updated successfully\n");
    write(STDOUT_FILENO, success.as_bytes());
    true
}
