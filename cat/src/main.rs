#![no_std]
#![no_main]

use libwunix::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};

const BUFFER_SIZE: usize = 1024;
const O_RDONLY: u64 = 0;

// Конкатенация и вывод содержимого файлов и потоков ввода (POSIX cat).
// Поддерживает:
// - передачу одного или нескольких файлов в аргументах (`cat file1 file2`)
// - чтение из STDIN при отсутствии аргументов или при указании дефиса '-'
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        cat_fd(STDIN_FILENO);
        libwunix::exit(0);
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

        if file_path == b"-" {
            cat_fd(STDIN_FILENO);
            continue;
        }

        let mut null_term = [0u8; 1024];
        if file_path.len() >= null_term.len() {
            libwunix::write(STDERR_FILENO, b"cat: file path too long\n");
            exit_code = 1;
            continue;
        }
        null_term[..file_path.len()].copy_from_slice(file_path);
        null_term[file_path.len()] = 0;

        let fd = libwunix::open(&null_term[..file_path.len() + 1], O_RDONLY, 0);
        if fd < 0 {
            libwunix::write(STDERR_FILENO, b"cat: ");
            libwunix::write(STDERR_FILENO, file_path);
            let err_msg = if fd == -13 || fd == -1 {
                libwunix::t(
                    ": Отказано в доступе\n",
                    ": Permission denied\n",
                )
            } else if fd == -21 {
                libwunix::t(
                    ": Это каталог\n",
                    ": Is a directory\n",
                )
            } else {
                libwunix::t(
                    ": Нет такого файла или каталога\n",
                    ": No such file or directory\n",
                )
            };
            libwunix::write(STDERR_FILENO, err_msg.as_bytes());
            exit_code = 1;
            continue;
        }

        cat_fd(fd as u64);
        let _ = libwunix::close(fd as u64);
    }

    libwunix::exit(exit_code);
}

fn cat_fd(fd: u64) {
    let mut buf = [0u8; BUFFER_SIZE];
    loop {
        let n = libwunix::read(fd, &mut buf);
        if n == 0 {
            break;
        }
        if (n as i64) < 0 {
            libwunix::write(
                STDERR_FILENO,
                libwunix::t("cat: ошибка чтения\n", "cat: read error\n").as_bytes(),
            );
            break;
        }
        let len = (n as usize).min(BUFFER_SIZE);
        if fd == STDIN_FILENO && len > 0 {
            if buf[0] == 0x03 {
                // Прерывание интерактивного ввода по Ctrl+C
                libwunix::write(STDOUT_FILENO, b"^C\n");
                libwunix::exit(130);
            }
            if buf[0] == 0x04 {
                // Конец файла (EOF) по Ctrl+D
                break;
            }
        }
        if let Some(slice) = buf.get(..len) {
            libwunix::write(STDOUT_FILENO, slice);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in cat\n");
    libwunix::exit(1);
}
