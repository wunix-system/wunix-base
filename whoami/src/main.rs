#![no_std]
#![no_main]

mod action;

use action::run_whoami;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: whoami [ОПЦИЯ]\n\
         Выводит имя пользователя, связанное с текущим эффективным идентификатором пользователя (EUID).\n\n\
         Опции:\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: whoami [OPTION]\n\
         Print the user name associated with the current effective user ID (EUID).\n\n\
         Options:\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 1 && !argv.is_null() {
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

    run_whoami();
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in whoami\n");
    exit(1);
}
