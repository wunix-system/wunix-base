#![no_std]
#![no_main]

mod action;
mod mode;

use action::change_file_mode;
use mode::parse_mode_spec;
use libwunix::{exit, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_help() {
    let help_msg = t(
        "Использование: chmod [ОПЦИЯ]... РЕЖИМ ФАЙЛ...\n\
         Изменяет права доступа каждого ФАЙЛА в соответствии с РЕЖИМОМ.\n\n\
         Примеры режимов:\n\
           755, 0644, 4755   восьмеричный режим\n\
           +x, -w, u+x, a+r  символьный режим\n\n\
         Опции:\n\
           -h, --help        показать эту справку и выйти\n",
        "Usage: chmod [OPTION]... MODE FILE...\n\
         Change the mode of each FILE to MODE.\n\n\
         Mode examples:\n\
           755, 0644, 4755   octal mode\n\
           +x, -w, u+x, a+r  symbolic mode\n\n\
         Options:\n\
           -h, --help        display this help and exit\n",
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
            "chmod: пропущен операнд\nПопробуйте 'chmod --help' для получения дополнительной информации.\n",
            "chmod: missing operand\nTry 'chmod --help' for more information.\n",
        );
        write(STDERR_FILENO, missing.as_bytes());
        exit(1);
    }

    let mode_ptr = unsafe { *argv.add(1) };
    let mode_len = str_utils::strlen(mode_ptr);
    let mode_str = core::str::from_utf8(unsafe { core::slice::from_raw_parts(mode_ptr, mode_len) }).unwrap_or("");

    let spec = match parse_mode_spec(mode_str) {
        Some(s) => s,
        None => {
            let invalid = t("chmod: неверный режим: '", "chmod: invalid mode: '");
            write(STDERR_FILENO, invalid.as_bytes());
            write(STDERR_FILENO, mode_str.as_bytes());
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
        if !change_file_mode(f_str, &spec) {
            had_error = true;
        }
    }

    exit(if had_error { 1 } else { 0 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in chmod\n");
    exit(1);
}
