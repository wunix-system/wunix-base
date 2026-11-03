#![no_std]
#![no_main]

use libwunix::{exit, mkdir, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

fn print_str(fd: u64, s: &str) {
    write(fd, s.as_bytes());
}

fn cstr_from_ptr(ptr: *const u8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let len = str_utils::strlen(ptr);
    core::str::from_utf8(unsafe { core::slice::from_raw_parts(ptr, len) }).unwrap_or("")
}

fn make_dir_p(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut buf = [0u8; 1024];
    let mut i = 0;

    // Пропускаем начальный слеш
    if bytes[0] == b'/' {
        buf[0] = b'/';
        i = 1;
    }

    while i < bytes.len() {
        if bytes[i] == b'/' {
            if i > 1 {
                buf[i] = 0;
                let _ = mkdir(&buf[..i + 1], 0o755);
            }
            buf[i] = b'/';
        } else {
            buf[i] = bytes[i];
        }
        i += 1;
    }

    buf[bytes.len()] = 0;
    let res = mkdir(&buf[..bytes.len() + 1], 0o755);
    res == 0 || res == -17 // EEXIST
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_str(
            STDERR_FILENO,
            t(
                "mkdir: пропущен операнд\nПопробуйте 'mkdir --help' для получения дополнительной информации.\n",
                "mkdir: missing operand\nTry 'mkdir --help' for more information.\n",
            ),
        );
        exit(1);
    }

    let mut create_parents = false;
    let mut paths_start = 1;

    for i in 1..argc {
        let arg = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if arg == "-p" || arg == "--parents" {
            create_parents = true;
        } else if arg == "--help" || arg == "-h" {
            print_str(
                STDOUT_FILENO,
                t(
                    "Использование: mkdir [КЛЮЧ]... КАТАЛОГ...\n\
                     Создаёт КАТАЛОГ(и), если они ещё не существуют.\n\n\
                     Обязательные аргументы для длинных ключей обязательны и для коротких.\n  \
                       -p, --parents     создавать родительские каталоги при необходимости\n  \
                       -h, --help        показать эту справку и выйти\n",
                    "Usage: mkdir [OPTION]... DIRECTORY...\n\
                     Create the DIRECTORY(ies), if they do not already exist.\n\n\
                     Mandatory arguments to long options are mandatory for short options too.\n  \
                       -p, --parents     no error if existing, make parent directories as needed\n  \
                       -h, --help        display this help and exit\n",
                ),
            );
            exit(0);
        } else {
            paths_start = i;
            break;
        }
    }

    let mut had_error = false;
    for i in paths_start..argc {
        let arg = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if arg == "-p" || arg == "--parents" {
            continue;
        }
        if arg.is_empty() {
            continue;
        }

        let success = if create_parents {
            make_dir_p(arg)
        } else {
            let mut null_term = [0u8; 1024];
            let bytes = arg.as_bytes();
            if bytes.len() >= 1023 {
                print_str(STDERR_FILENO, "mkdir: path too long\n");
                had_error = true;
                continue;
            }
            null_term[..bytes.len()].copy_from_slice(bytes);
            null_term[bytes.len()] = 0;
            mkdir(&null_term[..bytes.len() + 1], 0o755) == 0
        };

        if !success {
            print_str(STDERR_FILENO, t("mkdir: невозможно создать каталог '", "mkdir: cannot create directory '"));
            print_str(STDERR_FILENO, arg);
            print_str(STDERR_FILENO, t("': Ошибка создания\n", "': Failed to create\n"));
            had_error = true;
        }
    }

    exit(if had_error { 1 } else { 0 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    print_str(STDERR_FILENO, "panic in mkdir\n");
    exit(1);
}
