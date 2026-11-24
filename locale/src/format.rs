//! Модуль форматирования вывода утилиты locale в стандартном формате POSIX / macOS.

use libwunix::{write, STDOUT_FILENO};

/// Выводит текущие настройки переменных локали в стандартном формате POSIX.
pub fn print_locale_settings(
    lang: &str,
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    effective: &str,
) {
    let mut out = [0u8; 512];
    let mut cursor = 0;

    let append = |dst: &mut [u8], cur: &mut usize, s: &str| {
        let b = s.as_bytes();
        let len = b.len().min(dst.len() - *cur);
        dst[*cur..*cur + len].copy_from_slice(&b[..len]);
        *cur += len;
    };

    append(&mut out, &mut cursor, "LANG=\"");
    append(&mut out, &mut cursor, lang);
    append(&mut out, &mut cursor, "\"\n");

    for cat in &["LC_CTYPE", "LC_NUMERIC", "LC_TIME", "LC_COLLATE", "LC_MONETARY"] {
        append(&mut out, &mut cursor, cat);
        append(&mut out, &mut cursor, "=\"");
        append(&mut out, &mut cursor, effective);
        append(&mut out, &mut cursor, "\"\n");
    }

    append(&mut out, &mut cursor, "LC_MESSAGES=\"");
    if let Some(m) = lc_messages {
        append(&mut out, &mut cursor, m);
    } else {
        append(&mut out, &mut cursor, effective);
    }
    append(&mut out, &mut cursor, "\"\n");

    append(&mut out, &mut cursor, "LC_ALL=");
    if let Some(a) = lc_all {
        append(&mut out, &mut cursor, "\"");
        append(&mut out, &mut cursor, a);
        append(&mut out, &mut cursor, "\"");
    }
    append(&mut out, &mut cursor, "\n");

    write(STDOUT_FILENO, &out[..cursor]);
}

/// Выводит список доступных локалей.
pub fn print_locales_list(locales: &[&str]) {
    for &loc in locales {
        write(STDOUT_FILENO, loc.as_bytes());
        write(STDOUT_FILENO, b"\n");
    }
}

/// Выводит список поддерживаемых таблиц символов (charmaps).
pub fn print_charmaps() {
    write(STDOUT_FILENO, b"UTF-8\nASCII\n");
}

/// Выводит справку по использованию утилиты locale.
pub fn print_help() {
    let help = libwunix::t(
        "Использование: locale [ОПЦИИ]\n\
         Выводит информацию о текущей конфигурации локали или доступных локалях.\n\n\
         Опции:\n\
           -a, --all-locales    вывести список всех доступных локалей\n\
           -m, --charmaps       вывести список поддерживаемых кодировок (charmaps)\n\
           -h, --help           показать эту справку и выйти\n\
           -v, --version        показать информацию о версии и выйти\n",
        "Usage: locale [OPTIONS]\n\
         Output information about current locale or available locales.\n\n\
         Options:\n\
           -a, --all-locales    write names of available locales\n\
           -m, --charmaps       write names of available charmaps\n\
           -h, --help           display this help and exit\n\
           -v, --version        output version information and exit\n",
    );
    write(STDOUT_FILENO, help.as_bytes());
}

pub fn print_version() {
    write(STDOUT_FILENO, b"locale (Wunix coreutils) 0.1.0\n");
}
