use libwunix::{
    append_group_entry, append_passwd_entry, append_shadow_entry, chmod, chown, get_next_uid,
    get_passwd_by_name, geteuid, mkdir, t, write, PasswdEntry, STDERR_FILENO,
};

pub struct UserAddOptions<'a> {
    pub username: &'a str,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub home: Option<&'a str>,
    pub shell: Option<&'a str>,
    pub create_home: bool,
}

pub fn run_useradd(opts: UserAddOptions) -> bool {
    // 1. Проверка прав root
    if geteuid() != 0 {
        let err = t("useradd: только root может добавлять пользователей в систему\n", "useradd: only root may add a user to the system\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // 2. Проверка существования пользователя
    if let Ok(Some(_)) = get_passwd_by_name(opts.username) {
        let err_pre = t("useradd: пользователь '", "useradd: user '");
        let err_post = t("': уже существует\n", "': already exists\n");
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, opts.username.as_bytes());
        write(STDERR_FILENO, err_post.as_bytes());
        return false;
    }

    // 3. Определение UID и GID
    let target_uid = match opts.uid {
        Some(u) => u,
        None => match get_next_uid() {
            Ok(u) => u,
            Err(_) => 1000,
        },
    };
    let target_gid = opts.gid.unwrap_or(target_uid);

    // 4. Определение домашнего каталога и командной оболочки
    let mut home_buf = [0u8; 64];
    let home_str = if let Some(h) = opts.home {
        h
    } else {
        let prefix = b"/home/";
        let u_bytes = opts.username.as_bytes();
        let total = prefix.len() + u_bytes.len();
        if total < home_buf.len() {
            home_buf[..prefix.len()].copy_from_slice(prefix);
            home_buf[prefix.len()..total].copy_from_slice(u_bytes);
            core::str::from_utf8(&home_buf[..total]).unwrap_or("/home")
        } else {
            "/home"
        }
    };

    let shell_str = opts.shell.unwrap_or("/bin/wsh");

    // 5. Запись в /etc/passwd
    let entry = PasswdEntry::new(opts.username, target_uid, target_gid, opts.username, home_str, shell_str);
    if let Err(_) = append_passwd_entry(&entry) {
        let err = t("useradd: не удалось добавить запись в /etc/passwd\n", "useradd: failed to add entry to /etc/passwd\n");
        write(STDERR_FILENO, err.as_bytes());
        return false;
    }

    // 6. Запись в /etc/group (персональная группа пользователя)
    let _ = append_group_entry(opts.username, target_gid, "");

    // 7. Запись в /etc/shadow (по умолчанию заблокированный хеш "!")
    let _ = append_shadow_entry(opts.username, "!");

    // 8. Создание домашней директории (если create_home = true)
    if opts.create_home {
        let mut null_home = [0u8; 128];
        let h_b = home_str.as_bytes();
        if h_b.len() < null_home.len() - 1 {
            null_home[..h_b.len()].copy_from_slice(h_b);
            null_home[h_b.len()] = 0;
            let _ = mkdir(&null_home[..h_b.len() + 1], 0o755);
            let _ = chmod(&null_home[..h_b.len() + 1], 0o755);
            let _ = chown(&null_home[..h_b.len() + 1], target_uid, target_gid);
        }
    }

    true
}
