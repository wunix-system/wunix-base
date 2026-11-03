#![no_std]
#![no_main]

mod action;

use action::{run_useradd, UserAddOptions};
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: useradd [ОПЦИЯ]... ИМЯ_ПОЛЬЗОВАТЕЛЯ\n\
         Создает новую учетную запись пользователя и его домашний каталог.\n\n\
         Опции:\n\
           -m, --create-home  создать домашний каталог пользователя (по умолчанию включено)\n\
           -d, --home-dir ДИР указать домашний каталог\n\
           -s, --shell ШЕЛЛ   указать командную оболочку (по умолчанию /bin/wsh)\n\
           -u, --uid UID      указать конкретный числовой идентификатор пользователя\n\
           -h, --help         показать эту справку и выйти\n",
        "Usage: useradd [OPTION]... USERNAME\n\
         Create a new user account and home directory.\n\n\
         Options:\n\
           -m, --create-home  create the user's home directory (enabled by default)\n\
           -d, --home-dir DIR home directory for the new user\n\
           -s, --shell SHELL  login shell for the new user\n\
           -u, --uid UID      numerical user ID\n\
           -h, --help         display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

fn parse_u32(s: &str) -> Option<u32> {
    let mut val = 0u32;
    if s.is_empty() {
        return None;
    }
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        val = val.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(val)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        let missing = t(
            "useradd: пропущен операнд\nПопробуйте 'useradd --help' для получения дополнительной информации.\n",
            "useradd: missing operand\nTry 'useradd --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let mut username = None;
    let mut uid = None;
    let mut home = None;
    let mut shell = None;
    let mut create_home = true;

    let mut idx = 1;
    while idx < argc {
        let p = unsafe { *argv.add(idx as usize) };
        if p.is_null() {
            idx += 1;
            continue;
        }
        let len = str_utils::strlen(p);
        let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");

        if arg == "--help" || arg == "-h" {
            print_help();
            exit(0);
        } else if arg == "-m" || arg == "--create-home" {
            create_home = true;
        } else if (arg == "-d" || arg == "--home-dir") && idx + 1 < argc {
            idx += 1;
            let val_p = unsafe { *argv.add(idx as usize) };
            let val_len = str_utils::strlen(val_p);
            home = Some(core::str::from_utf8(unsafe { core::slice::from_raw_parts(val_p, val_len) }).unwrap_or(""));
        } else if (arg == "-s" || arg == "--shell") && idx + 1 < argc {
            idx += 1;
            let val_p = unsafe { *argv.add(idx as usize) };
            let val_len = str_utils::strlen(val_p);
            shell = Some(core::str::from_utf8(unsafe { core::slice::from_raw_parts(val_p, val_len) }).unwrap_or(""));
        } else if (arg == "-u" || arg == "--uid") && idx + 1 < argc {
            idx += 1;
            let val_p = unsafe { *argv.add(idx as usize) };
            let val_len = str_utils::strlen(val_p);
            let s = core::str::from_utf8(unsafe { core::slice::from_raw_parts(val_p, val_len) }).unwrap_or("");
            uid = parse_u32(s);
        } else if !arg.starts_with('-') {
            username = Some(arg);
        }
        idx += 1;
    }

    let user_str = match username {
        Some(u) => u,
        None => {
            let missing = t("useradd: не указано имя пользователя\n", "useradd: no username specified\n");
            write(STDERR_FILENO, missing.as_bytes());
            exit(1);
        }
    };

    let ok = run_useradd(UserAddOptions {
        username: user_str,
        uid,
        gid: None,
        home,
        shell,
        create_home,
    });

    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in useradd\n");
    exit(1);
}
