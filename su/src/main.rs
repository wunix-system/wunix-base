#![no_std]
#![no_main]

mod action;
mod terminal;

use action::run_su;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: su [ОПЦИЯ]... [ПОЛЬЗОВАТЕЛЬ]\n\
         Переключает текущий идентификатор пользователя на указанного ПОЛЬЗОВАТЕЛЯ (по умолчанию root).\n\n\
         Опции:\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: su [OPTION]... [USER]\n\
         Change the effective user ID and group ID to that of USER (default root).\n\n\
         Options:\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut target_user = "";

    if argc > 1 && !argv.is_null() {
        let p = unsafe { *argv.add(1) };
        if !p.is_null() {
            let len = str_utils::strlen(p);
            let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
            if arg == "--help" || arg == "-h" {
                print_help();
                exit(0);
            } else if !arg.starts_with('-') {
                target_user = arg;
            }
        }
    }

    let ok = run_su(target_user);
    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in su\n");
    exit(1);
}
