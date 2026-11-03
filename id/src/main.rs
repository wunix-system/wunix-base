#![no_std]
#![no_main]

mod action;
mod format;

use action::run_id;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: id [ОПЦИЯ]... [ПОЛЬЗОВАТЕЛЬ]\n\
         Выводит информацию о пользователе и группах для текущего пользователя или указанного ПОЛЬЗОВАТЕЛЯ.\n\n\
         Опции:\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: id [OPTION]... [USER]\n\
         Print user and group information for the current user, or for the specified USER.\n\n\
         Options:\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut specified_user = None;

    if argc > 1 && !argv.is_null() {
        let p = unsafe { *argv.add(1) };
        if !p.is_null() {
            let len = str_utils::strlen(p);
            let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
            if arg == "--help" || arg == "-h" {
                print_help();
                exit(0);
            } else if !arg.starts_with('-') {
                specified_user = Some(arg);
            }
        }
    }

    run_id(specified_user);
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in id\n");
    exit(1);
}
