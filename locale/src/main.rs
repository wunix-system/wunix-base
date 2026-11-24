//! Утилита locale для операционной системы Wunix.
//! Выводит параметры текущей локали и список доступных языковых пакетов.

#![no_std]
#![no_main]

mod action;
mod format;

use action::{show_all_locales, show_charmaps, show_current_locale};
use format::{print_help, print_version};
use libwunix::{exit, str_utils, write, STDERR_FILENO};

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    libwunix::init(argc, argv);
    if argc <= 1 || argv.is_null() {
        show_current_locale();
        exit(0);
    }

    let mut i = 1;
    while i < argc {
        let p = unsafe { *argv.add(i as usize) };
        if !p.is_null() {
            let len = str_utils::strlen(p);
            let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) })
                .unwrap_or("");

            match arg {
                "-a" | "--all-locales" => {
                    show_all_locales();
                    exit(0);
                }
                "-m" | "--charmaps" => {
                    show_charmaps();
                    exit(0);
                }
                "-h" | "--help" => {
                    print_help();
                    exit(0);
                }
                "-v" | "--version" => {
                    print_version();
                    exit(0);
                }
                _ => {
                    show_current_locale();
                    exit(0);
                }
            }
        }
        i += 1;
    }

    show_current_locale();
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in locale\n");
    exit(1);
}
