#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![cfg_attr(test, allow(dead_code, unused_imports))]

mod action;
mod terminal;
pub mod ticket;

use action::{run_reset_timestamp, run_sudo, run_validate};
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: sudo [ОПЦИЯ] КОМАНДА [АРГУМЕНТЫ...]\n\
         Исполняет команду от имени суперпользователя root.\n\n\
         Опции:\n\
           -h, --help             показать эту справку и выйти\n\
           -k, --reset-timestamp  сбросить сохранённые учетные данные (аннулировать билет)\n\
           -v, --validate         обновить сохранённый билет без запуска команды\n",
        "Usage: sudo [OPTION] COMMAND [ARGUMENTS...]\n\
         Execute a command as superuser root.\n\n\
         Options:\n\
           -h, --help             display this help and exit\n\
           -k, --reset-timestamp  invalidate timestamp ticket\n\
           -v, --validate         update timestamp ticket without running a command\n",
    );
    write(STDOUT_FILENO, help_msg.as_bytes());
}

#[cfg(not(test))]
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

    let mut arg_idx = 1;
    let mut invalidate_cached = false;

    while arg_idx < argc as usize {
        let p = unsafe { *argv.add(arg_idx) };
        let len = str_utils::strlen(p);
        let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");

        if arg == "--help" || arg == "-h" {
            print_help();
            exit(0);
        } else if arg == "-k" || arg == "--reset-timestamp" {
            invalidate_cached = true;
            arg_idx += 1;
        } else if arg == "-v" || arg == "--validate" {
            let ok = run_validate();
            exit(if ok { 0 } else { 1 });
        } else if arg == "--" {
            arg_idx += 1;
            break;
        } else {
            break;
        }
    }

    if invalidate_cached {
        run_reset_timestamp();
        if arg_idx >= argc as usize {
            exit(0);
        }
    }

    if arg_idx >= argc as usize {
        let missing = t(
            "sudo: пропущен операнд\nПопробуйте 'sudo --help' для получения дополнительной информации.\n",
            "sudo: missing operand\nTry 'sudo --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let p = unsafe { *argv.add(arg_idx) };
    let len = str_utils::strlen(p);
    let cmd = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");

    let ok = run_sudo(cmd, argv, arg_idx, argc as usize);
    exit(if ok { 0 } else { 1 });
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in sudo\n");
    exit(1);
}
