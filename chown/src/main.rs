#![no_std]
#![no_main]

mod action;
mod owner;

use action::change_file_owner;
use owner::parse_owner_spec;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: chown [ОПЦИЯ]... [ВЛАДЕЛЕЦ][:[ГРУППА]] ФАЙЛ...\n\
         Изменяет владельца и/или группу каждого ФАЙЛА на указанных ВЛАДЕЛЬЦА и/или ГРУППУ.\n\n\
         Примеры:\n\
           chown root /file          владелец root, группа не меняется\n\
           chown root:wheel /file   владелец root, группа wheel\n\
           chown :wheel /file        группа wheel, владелец не меняется\n\n\
         Опции:\n\
           -h, --help                показать эту справку и выйти\n",
        "Usage: chown [OPTION]... [OWNER][:[GROUP]] FILE...\n\
         Change the owner and/or group of each FILE to OWNER and/or GROUP.\n\n\
         Examples:\n\
           chown root /file          owner root, group unchanged\n\
           chown root:wheel /file   owner root, group wheel\n\
           chown :wheel /file        group wheel, owner unchanged\n\n\
         Options:\n\
           -h, --help                display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 2 || argv.is_null() {
        if argc == 2 {
            let p = unsafe { *argv.add(1) };
            if !p.is_null() {
                let len = str_utils::strlen(p);
                let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
                if arg == "--help" || arg == "-h" {
                    print_help();
                    exit(0);
                }
            }
        }
        let missing = t(
            "chown: пропущен операнд\nПопробуйте 'chown --help' для получения дополнительной информации.\n",
            "chown: missing operand\nTry 'chown --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let spec_ptr = unsafe { *argv.add(1) };
    let spec_len = str_utils::strlen(spec_ptr);
    let spec_str = core::str::from_utf8(unsafe { core::slice::from_raw_parts(spec_ptr, spec_len) }).unwrap_or("");

    let (target_uid, target_gid) = match parse_owner_spec(spec_str) {
        Ok(res) => res,
        Err(err) => {
            let invalid = if err == "invalid user" {
                t("chown: неверный пользователь: '", "chown: invalid user: '")
            } else {
                t("chown: неверная группа: '", "chown: invalid group: '")
            };
            write(STDERR_FILENO, invalid.as_bytes());
            write(STDERR_FILENO, spec_str.as_bytes());
            write(STDERR_FILENO, b"'\n");
            exit(1);
        }
    };

    let mut had_error = false;
    for i in 2..argc {
        let file_ptr = unsafe { *argv.add(i as usize) };
        if file_ptr.is_null() {
            continue;
        }
        let f_len = str_utils::strlen(file_ptr);
        let f_str = core::str::from_utf8(unsafe { core::slice::from_raw_parts(file_ptr, f_len) }).unwrap_or("");
        if !change_file_owner(f_str, target_uid, target_gid) {
            had_error = true;
        }
    }

    exit(if had_error { 1 } else { 0 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in chown\n");
    exit(1);
}
