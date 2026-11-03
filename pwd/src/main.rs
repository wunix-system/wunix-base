#![no_std]
#![no_main]

use libwunix::{STDERR_FILENO, STDOUT_FILENO};

const PATH_BUFFER_CAPACITY: usize = 256;

// Запрос текущей рабочей директории процесса через системный вызов getcwd.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut buf = [0u8; PATH_BUFFER_CAPACITY];
    let ret = libwunix::getcwd(&mut buf);
    if ret > 0 {
        let max_len = (ret as usize).min(PATH_BUFFER_CAPACITY);
        let mut len = max_len;
        for i in 0..max_len {
            if let Some(&b) = buf.get(i) {
                if b == 0 {
                    len = i;
                    break;
                }
            }
        }
        if let Some(slice) = buf.get(..len) {
            libwunix::write(STDOUT_FILENO, slice);
            libwunix::write(STDOUT_FILENO, b"\n");
        }
        libwunix::exit(0);
    } else {
        libwunix::write(
            STDERR_FILENO,
            libwunix::t(
                "pwd: не удалось получить текущую директорию из ядра\n",
                "pwd: error retrieving current directory from kernel\n",
            )
            .as_bytes(),
        );
        libwunix::exit(1);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in pwd\n");
    libwunix::exit(1);
}
