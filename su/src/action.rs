use crate::terminal::read_password_line;
use libwunix::{
    chdir, execve, get_passwd_by_name, getuid, setgid, setuid, t, verify_password, write,
    STDERR_FILENO, STDOUT_FILENO,
};

pub fn run_su(target_username: &str) -> bool {
    let target = if target_username.is_empty() {
        "root"
    } else {
        target_username
    };

    let user_entry = match get_passwd_by_name(target) {
        Ok(Some(e)) => e,
        _ => {
            let err_pre = t("su: пользователь '", "su: user '");
            let err_post = t("': не существует\n", "': does not exist\n");
            write(STDERR_FILENO, err_pre.as_bytes());
            write(STDERR_FILENO, target.as_bytes());
            write(STDERR_FILENO, err_post.as_bytes());
            return false;
        }
    };

    let real_uid = getuid();

    // Если текущий реальный пользователь не root, требуется проверка пароля целевого пользователя
    if real_uid != 0 {
        let shadow_entry = libwunix::get_shadow_by_name(target).ok().flatten();
        let needs_password = match &shadow_entry {
            Some(s) => !s.hash().is_empty(),
            None => true,
        };

        if needs_password {
            let prompt = t("Пароль: ", "Password: ");
            write(STDOUT_FILENO, prompt.as_bytes());

            let mut pwd_buf = [0u8; 128];
            let raw_pwd = read_password_line(&mut pwd_buf);

            if verify_password(target, raw_pwd).is_err() {
                let fail = t("su: Сбой аутентификации\n", "su: Authentication failure\n");
                write(STDERR_FILENO, fail.as_bytes());
                return false;
            }
        }
    }

    // Переключаем группу и пользователя
    let _ = setgid(user_entry.gid);
    let _ = setuid(user_entry.uid);

    // Переходим в домашнюю директорию целевого пользователя
    let home = user_entry.home();
    let mut null_home = [0u8; 128];
    let h_bytes = home.as_bytes();
    if h_bytes.len() < null_home.len() - 1 {
        null_home[..h_bytes.len()].copy_from_slice(h_bytes);
        null_home[h_bytes.len()] = 0;
        let _ = chdir(&null_home[..h_bytes.len() + 1]);
    }

    // Запускаем оболочку пользователя
    let shell = user_entry.shell();
    let shell_path = if shell.is_empty() { "/bin/wsh" } else { shell };

    let mut null_shell = [0u8; 128];
    let s_bytes = shell_path.as_bytes();
    if s_bytes.len() >= null_shell.len() - 1 {
        return false;
    }
    null_shell[..s_bytes.len()].copy_from_slice(s_bytes);
    null_shell[s_bytes.len()] = 0;

    let argv_ptrs = [null_shell.as_ptr(), core::ptr::null()];

    // Формируем окружение для нового шелла: USER, HOME, PATH, TERM
    let mut env_user = [0u8; 64];
    let u_bytes = target.as_bytes();
    let u_pre = b"USER=";
    env_user[..u_pre.len()].copy_from_slice(u_pre);
    env_user[u_pre.len()..u_pre.len() + u_bytes.len()].copy_from_slice(u_bytes);
    env_user[u_pre.len() + u_bytes.len()] = 0;

    let mut env_home = [0u8; 64];
    let h_pre = b"HOME=";
    env_home[..h_pre.len()].copy_from_slice(h_pre);
    env_home[h_pre.len()..h_pre.len() + h_bytes.len()].copy_from_slice(h_bytes);
    env_home[h_pre.len() + h_bytes.len()] = 0;

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
        &null_shell[..s_bytes.len() + 1],
        argv_ptrs.as_ptr() as u64,
        envp_ptrs.as_ptr() as u64,
    );

    if ret < 0 {
        let err = t("su: ошибка запуска оболочки\n", "su: failed to execute shell\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    true
}
