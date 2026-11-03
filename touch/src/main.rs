#![no_std]
#![no_main]

use libwunix::STDERR_FILENO;

const O_CREAT: u64 = 0x40;

// Создание пустых файлов или обновление времени модификации (POSIX touch).
// Принимает один или несколько путей к файлам для создания через O_CREAT.
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        libwunix::write(
            STDERR_FILENO,
            libwunix::t(
                "touch: пропущен операнд (файл)\nПопробуйте 'touch --help' для получения дополнительной информации.\n",
                "touch: missing file operand\nTry 'touch --help' for more information.\n",
            )
            .as_bytes(),
        );
        libwunix::exit(1);
    }

    let mut exit_code = 0;
    for i in 1..argc {
        let p = unsafe { *argv.add(i as usize) };
        if p.is_null() {
            continue;
        }

        let mut len = 0;
        unsafe {
            while *p.add(len) != 0 && len < 1024 {
                len += 1;
            }
        }
        let file_path = unsafe { core::slice::from_raw_parts(p, len) };

        let mut null_term = [0u8; 1024];
        if file_path.len() >= null_term.len() {
            libwunix::write(STDERR_FILENO, b"touch: file path too long\n");
            exit_code = 1;
            continue;
        }
        null_term[..file_path.len()].copy_from_slice(file_path);
        null_term[file_path.len()] = 0;

        let fd = libwunix::open(&null_term[..file_path.len() + 1], O_CREAT, 0);
        if fd >= 0 {
            let _ = libwunix::close(fd as u64);
        } else {
            libwunix::write(
                STDERR_FILENO,
                libwunix::t("touch: невозможно создать '", "touch: cannot touch '").as_bytes(),
            );
            libwunix::write(STDERR_FILENO, file_path);
            libwunix::write(
                STDERR_FILENO,
                libwunix::t("': ошибка создания файла\n", "': failed to create file\n").as_bytes(),
            );
            exit_code = 1;
        }
    }

    libwunix::exit(exit_code);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in touch\n");
    libwunix::exit(1);
}
