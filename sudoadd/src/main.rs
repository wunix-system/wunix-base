#![no_std]
#![no_main]

mod action;

use action::run_sudoadd;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: sudoadd [ОПЦИЯ] ИМЯ_ПОЛЬЗОВАТЕЛЯ\n\
         Добавляет пользователя в административную группу sudo (wheel).\n\n\
         Опции:\n\
           -d, --delete, -r, --remove  отозвать права sudo (удалить из группы wheel)\n\
           -h, --help                  показать эту справку и выйти\n\n\
         Примеры:\n\
           sudoadd alex       # предоставить пользователю alex права sudo\n\
           sudoadd -d alex    # отозвать права sudo у пользователя alex\n",
        "Usage: sudoadd [OPTION] USERNAME\n\
         Add a user to the administrative sudo group (wheel).\n\n\
         Options:\n\
           -d, --delete, -r, --remove  revoke sudo privileges (remove from wheel group)\n\
           -h, --help                  display this help and exit\n\n\
         Examples:\n\
           sudoadd alex       # grant sudo privileges to user alex\n\
           sudoadd -d alex    # revoke sudo privileges from user alex\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        let missing = t(
            "sudoadd: пропущен операнд\nПопробуйте 'sudoadd --help' для получения дополнительной информации.\n",
            "sudoadd: missing operand\nTry 'sudoadd --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let mut remove = false;
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
        } else if arg == "-d" || arg == "--delete" || arg == "-r" || arg == "--remove" {
            remove = true;
        } else if !arg.starts_with('-') {
            username = Some(arg);
        }
    }

    let user_str = match username {
        Some(u) => u,
        None => {
            let missing = t(
                "sudoadd: не указано имя пользователя\nПопробуйте 'sudoadd --help' для получения дополнительной информации.\n",
                "sudoadd: no username specified\nTry 'sudoadd --help' for more information.\n",
            );
            write(STDERR_FILENO, missing.as_bytes());
            exit(1);
        }
    };

    let ok = run_sudoadd(user_str, remove);
    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in sudoadd\n");
    exit(1);
}
