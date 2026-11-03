#![no_std]
#![no_main]

use libwunix::{STDERR_FILENO, STDOUT_FILENO};

// Стандартная утилита вывода строк и аргументов (POSIX echo).
// При наличии аргументов выводит их через пробел, завершая переводом строки.
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 1 && !argv.is_null() {
        for i in 1..argc {
            if i > 1 {
                libwunix::write(STDOUT_FILENO, b" ");
            }
            let p = unsafe { *argv.add(i as usize) };
            if !p.is_null() {
                let mut len = 0;
                unsafe {
                    while *p.add(len) != 0 && len < 4096 {
                        len += 1;
                    }
                    libwunix::write(STDOUT_FILENO, core::slice::from_raw_parts(p, len));
                }
            }
        }
    }
    libwunix::write(STDOUT_FILENO, b"\n");
    libwunix::exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in echo\n");
    libwunix::exit(1);
}
