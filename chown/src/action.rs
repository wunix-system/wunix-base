use libwunix::{chown, lstat, write, Stat, STDERR_FILENO};

pub fn change_file_owner(path: &str, target_uid: Option<u32>, target_gid: Option<u32>) -> bool {
    let mut null_path = [0u8; 1024];
    let bytes = path.as_bytes();
    if bytes.len() >= 1023 {
        let msg = libwunix::t("chown: путь слишком длинный\n", "chown: path too long\n");
        write(STDERR_FILENO, msg.as_bytes());
        return false;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let (final_uid, final_gid) = if target_uid.is_none() || target_gid.is_none() {
        let mut st = Stat::zeroed();
        if lstat(&null_path[..bytes.len() + 1], &mut st) != 0 {
            let err_pre = libwunix::t("chown: невозможно получить доступ к '", "chown: cannot access '");
            let err_post = libwunix::t("': Нет такого файла или каталога\n", "': No such file or directory\n");
            write(STDERR_FILENO, err_pre.as_bytes());
            write(STDERR_FILENO, path.as_bytes());
            write(STDERR_FILENO, err_post.as_bytes());
            return false;
        }
        (
            target_uid.unwrap_or(st.st_uid),
            target_gid.unwrap_or(st.st_gid),
        )
    } else {
        (target_uid.unwrap(), target_gid.unwrap())
    };

    let ret = chown(&null_path[..bytes.len() + 1], final_uid, final_gid);
    if ret < 0 {
        let err_pre = libwunix::t("chown: изменение владельца '", "chown: changing ownership of '");
        let err_post = if ret == -1 {
            libwunix::t("': Операция не позволена\n", "': Operation not permitted\n")
        } else {
            libwunix::t("': Отказано в доступе\n", "': Permission denied\n")
        };
        write(STDERR_FILENO, err_pre.as_bytes());
        write(STDERR_FILENO, path.as_bytes());
        write(STDERR_FILENO, err_post.as_bytes());
        return false;
    }

    true
}
