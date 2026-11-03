#![no_std]
#![no_main]

mod action;
mod terminal;

use action::run_sudo;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: sudo [ОПЦИЯ] КОМАНДА [АРГУМЕНТЫ...]\n\
         Исполняет команду от имени суперпользователя root.\n\n\
         Опции:\n\
           -h, --help     показать эту справку и выйти\n",
        "Usage: sudo [OPTION] COMMAND [ARGUMENTS...]\n\
         Execute a command as superuser root.\n\n\
         Options:\n\
           -h, --help     display this help and exit\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        let missing = t(
            "sudo: пропущен операнд\nПопробуйте 'sudo --help' для получения дополнительной информации.\n",
            "sudo: missing operand\nTry 'sudo --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let p = unsafe { *argv.add(1) };
    let len = str_utils::strlen(p);
    let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");

    if arg == "--help" || arg == "-h" {
        print_help();
        exit(0);
    }

    let ok = run_sudo(arg, argv, 1, argc as usize);
    exit(if ok { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in sudo\n");
    exit(1);
}
