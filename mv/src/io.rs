use libwunix::{t, write, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};

pub fn print_out(msg: &str) {
    write(STDOUT_FILENO, msg.as_bytes());
}

pub fn print_err(msg: &str) {
    write(STDERR_FILENO, msg.as_bytes());
}

pub fn prompt_overwrite(dst: &[u8]) -> bool {
    print_err(t("mv: перезаписать '", "mv: overwrite '"));
    write(STDERR_FILENO, dst);
    print_err("'? ");

    let mut buf = [0u8; 64];
    let n = libwunix::read(STDIN_FILENO, &mut buf) as i64;
    if n <= 0 {
        return false;
    }

    let input = &buf[..n as usize];
    matches!(input.first(), Some(b'y' | b'Y'))
        || input.starts_with("д".as_bytes())
        || input.starts_with("Д".as_bytes())
}

pub fn print_move_error(src: &[u8], dst: &[u8], errno_neg: i64) {
    print_err(t("mv: не удалось переместить '", "mv: cannot move '"));
    write(STDERR_FILENO, src);
    print_err(t("' в '", "' to '"));
    write(STDERR_FILENO, dst);
    print_err("': ");
    print_errno(errno_neg);
    print_err("\n");
}

pub fn print_errno(errno_neg: i64) {
    let code = (-errno_neg) as i32;
    let msg = match code {
        1 => t("Операция не позволена", "Operation not permitted"),
        2 => t("Нет такого файла или каталога", "No such file or directory"),
        5 => t("Ошибка ввода/вывода", "Input/output error"),
        13 => t("Отказано в доступе", "Permission denied"),
        16 => t("Устройство или ресурс занято", "Device or resource busy"),
        17 => t("Файл существует", "File exists"),
        18 => t("Недопустимая ссылка между устройствами", "Invalid cross-device link"),
        20 => t("Это не каталог", "Not a directory"),
        21 => t("Это каталог", "Is a directory"),
        22 => t("Недопустимый аргумент", "Invalid argument"),
        39 => t("Каталог не пуст", "Directory not empty"),
        _ => t("Системная ошибка", "System error"),
    };
    print_err(msg);
}
