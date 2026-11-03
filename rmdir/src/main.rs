#![no_std]
#![no_main]

use libwunix::{exit, rmdir, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO};

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

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_str(
            STDERR_FILENO,
            t(
                "rmdir: пропущен операнд\nПопробуйте 'rmdir --help' для получения дополнительной информации.\n",
                "rmdir: missing operand\nTry 'rmdir --help' for more information.\n",
            ),
        );
        exit(1);
    }

    let mut had_error = false;
    for i in 1..argc {
        let arg = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if arg == "--help" || arg == "-h" {
            print_str(
                STDOUT_FILENO,
                t(
                    "Использование: rmdir [КЛЮЧ]... КАТАЛОГ...\n\
                     Удаляет пустой КАТАЛОГ(и).\n\n\
                       -h, --help        показать эту справку и выйти\n",
                    "Usage: rmdir [OPTION]... DIRECTORY...\n\
                     Remove the DIRECTORY(ies), if they are empty.\n\n\
                       -h, --help        display this help and exit\n",
                ),
            );
            exit(0);
        }

        if arg.is_empty() {
            continue;
        }

        let mut null_term = [0u8; 1024];
        let bytes = arg.as_bytes();
        if bytes.len() >= 1023 {
            print_str(STDERR_FILENO, "rmdir: path too long\n");
            had_error = true;
            continue;
        }
        null_term[..bytes.len()].copy_from_slice(bytes);
        null_term[bytes.len()] = 0;

        let ret = rmdir(&null_term[..bytes.len() + 1]);
        if ret != 0 {
            print_str(STDERR_FILENO, t("rmdir: не удалось удалить '", "rmdir: failed to remove '"));
            print_str(STDERR_FILENO, arg);
            let msg = match ret {
                -39 => t("': Каталог не пуст\n", "': Directory not empty\n"),
                -2 => t("': Нет такого файла или каталога\n", "': No such file or directory\n"),
                -20 => t("': Не является каталогом\n", "': Not a directory\n"),
                -13 => t("': Отказано в доступе\n", "': Permission denied\n"),
                _ => t("': Ошибка удаления\n", "': Removal failed\n"),
            };
            print_str(STDERR_FILENO, msg);
            had_error = true;
        }
    }

    exit(if had_error { 1 } else { 0 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    print_str(STDERR_FILENO, "panic in rmdir\n");
    exit(1);
}
