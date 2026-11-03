#![no_std]
#![no_main]

use libwunix::{STDERR_FILENO, STDOUT_FILENO};

// Очистка экрана консоли.
// Отправляет ANSI escape-последовательность:
// \x1b[2J — очистить весь экран,
// \x1b[H  — перевести курсор в верхний левый угол (строка 1, столбец 1).
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    libwunix::write(STDOUT_FILENO, b"\x1b[2J\x1b[H");
    libwunix::exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in clear\n");
    libwunix::exit(1);
}
