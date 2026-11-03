use crate::mode::{apply_symbolic_mode, ModeSpec};
use libwunix::{chmod, lstat, write, Stat, STDERR_FILENO};

pub fn change_file_mode(path: &str, spec: &ModeSpec) -> bool {
    let mut null_path = [0u8; 1024];
    let bytes = path.as_bytes();
    if bytes.len() >= 1023 {
        let msg = libwunix::t("chmod: путь слишком длинный\n", "chmod: path too long\n");
        write(STDERR_FILENO, msg.as_bytes());
        return false;
    }
    null_path[..bytes.len()].copy_from_slice(bytes);
    null_path[bytes.len()] = 0;

    let target_mode = match spec {
        ModeSpec::Octal(m) => *m,
        ModeSpec::Symbolic(s) => {
            let mut st = Stat::zeroed();
            if lstat(&null_path[..bytes.len() + 1], &mut st) != 0 {
                let err_pre = libwunix::t("chmod: невозможно получить доступ к '", "chmod: cannot access '");
                let err_post = libwunix::t("': Нет такого файла или каталога\n", "': No such file or directory\n");
                write(STDERR_FILENO, err_pre.as_bytes());
                write(STDERR_FILENO, path.as_bytes());
                write(STDERR_FILENO, err_post.as_bytes());
                return false;
            }
            let cur = st.st_mode & 0o7777;
            match apply_symbolic_mode(cur, s) {
                Some(m) => m,
                None => {
                    let err = libwunix::t("chmod: неверный символьный режим\n", "chmod: invalid symbolic mode\n");
                    write(STDERR_FILENO, err.as_bytes());
                    return false;
                }
            }
        }
    };

    let ret = chmod(&null_path[..bytes.len() + 1], target_mode);
    if ret < 0 {
        let err_pre = libwunix::t("chmod: изменение прав для '", "chmod: changing permissions of '");
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
