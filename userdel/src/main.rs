#![no_std]
#![no_main]

mod action;

use action::run_userdel;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: userdel [ОПЦИЯ]... ИМЯ_ПОЛЬЗОВАТЕЛЯ\n\
         Удаляет учетную запись пользователя и связанные с ней файлы.\n\n\
         Опции:\n\
           -r, --remove   удалить домашний каталог и почтовый ящик пользователя\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: userdel [OPTION]... USERNAME\n\
         Delete a user account and related files.\n\n\
         Options:\n\
           -r, --remove   remove home directory and mail spool\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        let missing = t(
            "userdel: пропущен операнд\nПопробуйте 'userdel --help' для получения дополнительной информации.\n",
            "userdel: missing operand\nTry 'userdel --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let mut remove_home = false;
    let mut username = None;

    for i in 1..argc {
        let p = unsafe { *argv.add(i as usize) };
        if p.is_null() {
            continue;
        }
        let len = str_utils::strlen(p);
        let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");

        if arg == "--help" || arg == "-h" {
            print_help();
            exit(0);
        } else if arg == "-r" || arg == "--remove" {
            remove_home = true;
        } else if !arg.starts_with('-') {
            username = Some(arg);
        }
    }

    let user_str = match username {
        Some(u) => u,
        None => {
            let missing = t("userdel: не указано имя пользователя\n", "userdel: no username specified\n");
            write(STDERR_FILENO, missing.as_bytes());
            exit(1);
        }
    };

    let ok = run_userdel(user_str, remove_home);
    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in userdel\n");
    exit(1);
}
