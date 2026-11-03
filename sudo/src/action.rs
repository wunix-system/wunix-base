use crate::terminal::read_password_line;
use libwunix::{
    execve, get_passwd_by_uid, getuid, setgid, setuid, t, user_in_group, verify_password, write,
    STDERR_FILENO, STDOUT_FILENO,
};

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

    // 2. Аутентификация (запрос пароля текущего пользователя)
    if real_uid != 0 {
        let prompt_pre = t("[sudo] пароль для ", "[sudo] password for ");
        let prompt_post = ": ";
        write(STDOUT_FILENO, prompt_pre.as_bytes());
        write(STDOUT_FILENO, username.as_bytes());
        write(STDOUT_FILENO, prompt_post.as_bytes());

        let mut pwd_buf = [0u8; 128];
        let raw_pwd = read_password_line(&mut pwd_buf);

        if verify_password(username, raw_pwd).is_err() {
            let fail = t("sudo: неверный пароль\n", "sudo: Authentication failure: incorrect password\n");
            write(STDERR_FILENO, fail.as_bytes());
            return false;
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
