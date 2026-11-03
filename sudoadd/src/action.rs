use libwunix::{
    add_user_to_group, get_passwd_by_name, geteuid, remove_user_from_group, t, user_in_group,
    write, AuthError, STDERR_FILENO, STDOUT_FILENO,
};

/// Выполняет добавление или отзыв привилегий sudo (членство в группе wheel).
pub fn run_sudoadd(username: &str, remove: bool) -> bool {
    // Управление правами sudo — критическая административная операция,
    // непривилегированные процессы не должны иметь возможность менять состав wheel.
    if geteuid() != 0 {
        let err = t(
            "sudoadd: только root может управлять правами sudo\n",
            "sudoadd: only root may manage sudo privileges\n",
        );
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // Проверяем наличие учетной записи в /etc/passwd перед манипуляциями с группами
    if let Ok(None) = get_passwd_by_name(username) {
        let pre = t("sudoadd: пользователь '", "sudoadd: user '");
        let post = t("' не найден в системе\n", "' not found in system\n");
        write(STDERR_FILENO, pre.as_bytes());
        write(STDERR_FILENO, username.as_bytes());
        write(STDERR_FILENO, post.as_bytes());
        return false;
    }

    if remove {
        match remove_user_from_group(username, "wheel") {
            Ok(()) => {
                let pre = t("sudoadd: пользователь '", "sudoadd: user '");
                let post = t(
                    "' успешно удален из группы sudo (wheel)\n",
                    "' successfully removed from sudo (wheel group)\n",
                );
                write(STDOUT_FILENO, pre.as_bytes());
                write(STDOUT_FILENO, username.as_bytes());
                write(STDOUT_FILENO, post.as_bytes());
                true
            }
            Err(AuthError::UserNotFound) => {
                let pre = t("sudoadd: пользователь '", "sudoadd: user '");
                let post = t(
                    "' не состоит в группе sudo (wheel)\n",
                    "' is not a member of sudo (wheel group)\n",
                );
                write(STDERR_FILENO, pre.as_bytes());
                write(STDERR_FILENO, username.as_bytes());
                write(STDERR_FILENO, post.as_bytes());
                false
            }
            Err(AuthError::GroupNotFound) => {
                let err = t(
                    "sudoadd: группа 'wheel' не найдена в /etc/group\n",
                    "sudoadd: group 'wheel' not found in /etc/group\n",
                );
                write(STDERR_FILENO, err.as_bytes());
                false
            }
            Err(_) => {
                let err = t(
                    "sudoadd: не удалось обновить /etc/group\n",
                    "sudoadd: failed to update /etc/group\n",
                );
                write(STDERR_FILENO, err.as_bytes());
                false
            }
        }
    } else {
        // Если пользователь уже в wheel, операция идемпотентна — сообщаем и возвращаем успех
        if user_in_group(username, "wheel") {
            let pre = t("sudoadd: пользователь '", "sudoadd: user '");
            let post = t(
                "' уже имеет права sudo (состоит в группе wheel)\n",
                "' already has sudo privileges (member of wheel group)\n",
            );
            write(STDOUT_FILENO, pre.as_bytes());
            write(STDOUT_FILENO, username.as_bytes());
            write(STDOUT_FILENO, post.as_bytes());
            return true;
        }

        match add_user_to_group(username, "wheel") {
            Ok(()) => {
                let pre = t("sudoadd: пользователь '", "sudoadd: user '");
                let post = t(
                    "' успешно добавлен в группу sudo (wheel)\n",
                    "' successfully added to sudo (wheel group)\n",
                );
                write(STDOUT_FILENO, pre.as_bytes());
                write(STDOUT_FILENO, username.as_bytes());
                write(STDOUT_FILENO, post.as_bytes());
                true
            }
            Err(AuthError::AlreadyExists) => {
                let pre = t("sudoadd: пользователь '", "sudoadd: user '");
                let post = t(
                    "' уже состоит в группе sudo (wheel)\n",
                    "' is already in sudo (wheel group)\n",
                );
                write(STDOUT_FILENO, pre.as_bytes());
                write(STDOUT_FILENO, username.as_bytes());
                write(STDOUT_FILENO, post.as_bytes());
                true
            }
            Err(AuthError::GroupNotFound) => {
                let err = t(
                    "sudoadd: группа 'wheel' не найдена в /etc/group\n",
                    "sudoadd: group 'wheel' not found in /etc/group\n",
                );
                write(STDERR_FILENO, err.as_bytes());
                false
            }
            Err(_) => {
                let err = t(
                    "sudoadd: не удалось обновить /etc/group\n",
                    "sudoadd: failed to update /etc/group\n",
                );
                write(STDERR_FILENO, err.as_bytes());
                false
            }
        }
    }
}
