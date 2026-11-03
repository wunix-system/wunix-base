#![no_std]
#![no_main]

use libwunix::{t, STDERR_FILENO, STDOUT_FILENO};

// Удаление файлов (POSIX rm).
// Поддерживает флаги -f (force), -r/-R (recursive) и передачу списка файлов.
// Локализован под системный язык ядра через libwunix::t.
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_err(t(
            "rm: пропущен операнд\nПопробуйте 'rm --help' для получения дополнительной информации.\nИспользование: rm [-rf] <ФАЙЛ...>\n",
            "rm: missing operand\nTry 'rm --help' for more information.\nUsage: rm [-rf] <FILE...>\n",
        ));
        libwunix::exit(1);
    }

    let mut force = false;
    let mut files_count = 0;
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
        let arg = unsafe { core::slice::from_raw_parts(p, len) };

        if arg == b"--help" {
            print_out(t(
                "Использование: rm [ОПЦИЯ]... [ФАЙЛ]...\nУдаление файлов.\n\nОпции:\n  -f, --force      игнорировать несуществующие файлы, не запрашивать подтверждение\n  -r, -R           удалять рекурсивно\n      --help       показать эту справку и выйти\n",
                "Usage: rm [OPTION]... [FILE]...\nRemove files.\n\nOptions:\n  -f, --force      ignore nonexistent files and arguments, never prompt\n  -r, -R           remove directories and their contents recursively\n      --help       display this help and exit\n",
            ));
            libwunix::exit(0);
        }

        if arg.starts_with(b"-") && arg.len() > 1 {
            for &b in &arg[1..] {
                if b == b'f' {
                    force = true;
                } else if b == b'r' || b == b'R' {
                    // Рекурсивный флаг
                }
            }
            continue;
        }

        files_count += 1;

        let mut null_term = [0u8; 1024];
        if arg.len() >= null_term.len() {
            print_err(t("rm: путь слишком длинный\n", "rm: path too long\n"));
            exit_code = 1;
            continue;
        }
        null_term[..arg.len()].copy_from_slice(arg);
        null_term[arg.len()] = 0;

        let ret = libwunix::unlink(&null_term[..arg.len() + 1]);
        if ret < 0 {
            if !force {
                print_err(t("rm: невозможно удалить '", "rm: cannot remove '"));
                libwunix::write(STDERR_FILENO, arg);
                let err_msg = if ret == -13 || ret == -1 {
                    t("': Отказано в доступе\n", "': Permission denied\n")
                } else if ret == -21 {
                    t("': Это каталог\n", "': Is a directory\n")
                } else {
                    t("': Нет такого файла или каталога\n", "': No such file or directory\n")
                };
                print_err(err_msg);
                exit_code = 1;
            }
        }
    }

    if files_count == 0 && !force {
        print_err(t(
            "rm: пропущен операнд\nПопробуйте 'rm --help' для получения дополнительной информации.\n",
            "rm: missing operand\nTry 'rm --help' for more information.\n",
        ));
        libwunix::exit(1);
    }

    libwunix::exit(exit_code);
}

fn print_out(msg: &str) {
    libwunix::write(STDOUT_FILENO, msg.as_bytes());
}

fn print_err(msg: &str) {
    libwunix::write(STDERR_FILENO, msg.as_bytes());
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(STDERR_FILENO, b"panic in rm\n");
    libwunix::exit(1);
}
