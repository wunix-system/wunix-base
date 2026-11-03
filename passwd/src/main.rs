#![no_std]
#![no_main]

mod action;
mod terminal;
mod time;

use action::run_passwd;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: passwd [ОПЦИЯ]... [ПОЛЬЗОВАТЕЛЬ]\n\
         Обновляет маркер аутентификации (пароль) пользователя в /etc/shadow.\n\n\
         Опции:\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: passwd [OPTION]... [USER]\n\
         Update user's authentication tokens (password) in /etc/shadow.\n\n\
         Options:\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut target_user = None;

    if argc > 1 && !argv.is_null() {
        let p = unsafe { *argv.add(1) };
        if !p.is_null() {
            let len = str_utils::strlen(p);
            let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
            if arg == "--help" || arg == "-h" {
                print_help();
                exit(0);
            } else if !arg.starts_with('-') {
                target_user = Some(arg);
            }
        }
    }

    let ok = run_passwd(target_user);
    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in passwd\n");
    exit(1);
}
