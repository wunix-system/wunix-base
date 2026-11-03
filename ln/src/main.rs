#![no_std]
#![no_main]

use libwunix::{exit, str_utils, symlink, t, unlink, write, STDERR_FILENO, STDOUT_FILENO};

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
                "ln: пропущен операнд\nПопробуйте 'ln --help' для получения дополнительной информации.\n",
                "ln: missing operand\nTry 'ln --help' for more information.\n",
            ),
        );
        exit(1);
    }

    let prog_name = cstr_from_ptr(unsafe { *argv });
    let is_mklink = prog_name.ends_with("mklink") || prog_name.ends_with("mklink.exe");

    let mut is_symbolic = is_mklink;
    let mut force = false;
    let mut args: [Option<&str>; 4] = [None; 4];
    let mut arg_idx = 0;

    for i in 1..argc {
        let val = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if val == "-s" || val == "--symbolic" || val == "/D" || val == "/d" {
            is_symbolic = true;
        } else if val == "-f" || val == "--force" {
            force = true;
        } else if val == "-sf" || val == "-fs" {
            is_symbolic = true;
            force = true;
        } else if val == "/H" || val == "/h" && is_mklink {
            is_symbolic = false;
        } else if val == "--help" || val == "-h" || val == "/?" {
            print_str(
                STDOUT_FILENO,
                t(
                    "Использование: ln [КЛЮЧ]... ЦЕЛЬ [ИМЯ_ССЫЛКИ]\n\
                     Создаёт ссылку на ЦЕЛЬ с ИМЕНЕМ_ССЫЛКИ.\n\n\
                       -s, --symbolic    создавать символические ссылки вместо жестких\n  \
                       -f, --force       удалять существующие целевые файлы перед созданием\n  \
                       -h, --help        показать эту справку и выйти\n",
                    "Usage: ln [OPTION]... TARGET [LINK_NAME]\n\
                     Create a link to TARGET with the name LINK_NAME.\n\n\
                       -s, --symbolic    make symbolic links instead of hard links\n  \
                       -f, --force       remove existing destination files\n  \
                       -h, --help        display this help and exit\n",
                ),
            );
            exit(0);
        } else if arg_idx < args.len() {
            args[arg_idx] = Some(val);
            arg_idx += 1;
        }
    }

    let (target, link_name) = if is_mklink {
        // Альтернативный синтаксис команды mklink: mklink [/D] Ссылка Цель
        match (args[0], args[1]) {
            (Some(l), Some(t)) => (t, l),
            (Some(_), None) | (None, _) => {
                print_str(
                    STDERR_FILENO,
                    t(
                        "Синтаксис команды: mklink [[/D] | [/H] | [/J]] Ссылка Цель\n",
                        "Command syntax: mklink [[/D] | [/H] | [/J]] Link Target\n",
                    ),
                );
                exit(1);
            }
        }
    } else {
        let t = match args[0] {
            Some(t) => t,
            None => {
                print_str(STDERR_FILENO, t("ln: не указана цель ссылки\n", "ln: missing target\n"));
                exit(1);
            }
        };

        let l = match args[1] {
            Some(l) => l,
            None => {
                if let Some(idx) = t.rfind('/') {
                    &t[idx + 1..]
                } else {
                    t
                }
            }
        };
        (t, l)
    };

    if !is_symbolic {
        print_str(
            STDERR_FILENO,
            t(
                "ln: создание жестких ссылок не поддерживается файловой системой; используйте '-s'\n",
                "ln: hard links not supported by filesystem; use '-s'\n",
            ),
        );
        exit(1);
    }

    let mut target_null = [0u8; 1024];
    let t_bytes = target.as_bytes();
    if t_bytes.len() >= 1023 {
        print_str(STDERR_FILENO, "ln: target path too long\n");
        exit(1);
    }
    target_null[..t_bytes.len()].copy_from_slice(t_bytes);
    target_null[t_bytes.len()] = 0;

    let mut link_null = [0u8; 1024];
    let l_bytes = link_name.as_bytes();
    if l_bytes.len() >= 1023 {
        print_str(STDERR_FILENO, "ln: link path too long\n");
        exit(1);
    }
    link_null[..l_bytes.len()].copy_from_slice(l_bytes);
    link_null[l_bytes.len()] = 0;

    if force {
        let _ = unlink(&link_null[..l_bytes.len() + 1]);
    }

    let ret = symlink(&target_null[..t_bytes.len() + 1], &link_null[..l_bytes.len() + 1]);
    if ret != 0 {
        print_str(STDERR_FILENO, t("ln: не удалось создать ссылку '", "ln: failed to create link '"));
        print_str(STDERR_FILENO, link_name);
        let msg = match ret {
            -17 => t("': Файл уже существует\n", "': File exists\n"),
            -2 => t("': Нет такого файла или каталога\n", "': No such file or directory\n"),
            -13 => t("': Отказано в доступе\n", "': Permission denied\n"),
            _ => t("': Ошибка создания ссылки\n", "': Link creation failed\n"),
        };
        print_str(STDERR_FILENO, msg);
        exit(1);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    print_str(STDERR_FILENO, "panic in ln\n");
    exit(1);
}
