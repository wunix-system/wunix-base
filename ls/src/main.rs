#![no_std]
#![no_main]

mod format;

use format::{
    print_colored_name, print_long_entry, DT_DIR, DT_LNK, DT_REG,
};
use libwunix::{
    close, exit, getdents64, lstat, open, str_utils, t, write, Stat, STDERR_FILENO,
    STDOUT_FILENO,
};

const BUFFER_SIZE: usize = 4096;
const O_RDONLY: u64 = 0;

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

fn print_help() {
    print_str(
        STDOUT_FILENO,
        t(
            "Использование: ls [КЛЮЧ]... [ФАЙЛ]...\n\
             Выводит информацию о ФАЙЛАХ (по умолчанию о текущем каталоге).\n\n\
               -a, --all        не скрывать файлы, начинающиеся с .\n  \
               -l               использовать подробный формат вывода\n  \
               -h, --help       показать эту справку и выйти\n",
            "Usage: ls [OPTION]... [FILE]...\n\
             List information about the FILEs (the current directory by default).\n\n\
               -a, --all        do not ignore entries starting with .\n  \
               -l               use a long listing format\n  \
               -h, --help       display this help and exit\n",
        ),
    );
}

fn list_directory(target: &str, show_all: bool, long_format: bool) -> bool {
    let mut null_target = [0u8; 1024];
    let t_bytes = target.as_bytes();
    if t_bytes.len() >= 1023 {
        print_str(STDERR_FILENO, "ls: path too long\n");
        return false;
    }
    null_target[..t_bytes.len()].copy_from_slice(t_bytes);
    null_target[t_bytes.len()] = 0;

    let mut st = Stat::zeroed();
    if lstat(&null_target[..t_bytes.len() + 1], &mut st) == 0 {
        if (st.st_mode & 0o170000) != 0o040000 {
            // Аргумент не является директорией — выводим одиночный файл
            list_single_file(target, long_format);
            return true;
        }
    }

    let fd = open(&null_target[..t_bytes.len() + 1], O_RDONLY, 0);
    if fd < 0 {
        return false;
    }

    let mut buf = [0u8; BUFFER_SIZE];
    let mut any_printed = false;
    let mut is_first_chunk = true;

    loop {
        let n = getdents64(fd as u64, &mut buf);
        if n <= 0 {
            if (n == -20 || n == -9) && is_first_chunk {
                // ENOTDIR: аргумент является файлом
                let _ = close(fd as u64);
                list_single_file(target, long_format);
                return true;
            }
            break;
        }
        is_first_chunk = false;

        let total = (n as usize).min(BUFFER_SIZE);
        let mut pos = 0;
        while pos + 19 <= total {
            let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
            if reclen == 0 || pos + reclen > total {
                break;
            }

            let dtype = buf[pos + 18];
            let name_start = pos + 19;
            let mut name_end = name_start;
            let max_end = pos + reclen;
            while name_end < max_end && buf[name_end] != 0 {
                name_end += 1;
            }

            if name_end > name_start {
                if let Ok(name) = core::str::from_utf8(&buf[name_start..name_end]) {
                    if show_all || !name.starts_with('.') {
                        any_printed = true;
                        if long_format {
                            let mut full_path = [0u8; 1024];
                            let mut f_len = 0;
                            if target != "." {
                                let copy_t = t_bytes.len().min(full_path.len() - 1);
                                full_path[..copy_t].copy_from_slice(&t_bytes[..copy_t]);
                                f_len += copy_t;
                                if !target.ends_with('/') && f_len < full_path.len() - 1 {
                                    full_path[f_len] = b'/';
                                    f_len += 1;
                                }
                            }
                            let copy_n = name.len().min(full_path.len().saturating_sub(f_len) - 1);
                            full_path[f_len..f_len + copy_n].copy_from_slice(&name.as_bytes()[..copy_n]);
                            f_len += copy_n;
                            full_path[f_len] = 0;

                            let mut st = Stat::zeroed();
                            let (size, mode, uid, gid) = if lstat(&full_path[..f_len + 1], &mut st) == 0 {
                                (st.st_size as u64, st.st_mode, st.st_uid, st.st_gid)
                            } else {
                                let default_mode = match dtype {
                                    DT_DIR => 0o040755,
                                    DT_LNK => 0o120777,
                                    _ => 0o100644,
                                };
                                (0, default_mode, 0, 0)
                            };

                            print_long_entry(target, name, dtype, size, mode, uid, gid);
                        } else {
                            let is_exec = format::is_executable(name, 0);
                            print_colored_name(name, dtype, is_exec);
                            write(STDOUT_FILENO, b"  ");
                        }
                    }
                }
            }

            pos += reclen;
        }
    }

    let _ = close(fd as u64);

    if !long_format && any_printed {
        write(STDOUT_FILENO, b"\n");
    }

    true
}

fn list_single_file(file: &str, long_format: bool) {
    let mut null_file = [0u8; 1024];
    let f_bytes = file.as_bytes();
    let copy_f = f_bytes.len().min(null_file.len() - 1);
    null_file[..copy_f].copy_from_slice(f_bytes);
    null_file[copy_f] = 0;

    let mut st = Stat::zeroed();
    let res = lstat(&null_file[..copy_f + 1], &mut st);

    let (dtype, size, mode, uid, gid) = if res == 0 {
        let dt = if (st.st_mode & 0o170000) == 0o040000 {
            DT_DIR
        } else if (st.st_mode & 0o170000) == 0o120000 {
            DT_LNK
        } else {
            DT_REG
        };
        (dt, st.st_size as u64, st.st_mode, st.st_uid, st.st_gid)
    } else {
        (DT_REG, 0, 0o100644, 0, 0)
    };

    if long_format {
        print_long_entry(".", file, dtype, size, mode, uid, gid);
    } else {
        let is_exec = format::is_executable(file, mode);
        print_colored_name(file, dtype, is_exec);
        write(STDOUT_FILENO, b"\n");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut show_all = false;
    let mut long_format = false;
    let mut targets: [Option<&str>; 16] = [None; 16];
    let mut target_count = 0;

    for i in 1..argc {
        let val = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if val == "-a" || val == "--all" {
            show_all = true;
        } else if val == "-l" {
            long_format = true;
        } else if val == "-la" || val == "-al" {
            show_all = true;
            long_format = true;
        } else if val == "-h" || val == "--help" {
            print_help();
            exit(0);
        } else if !val.starts_with('-') && target_count < targets.len() {
            targets[target_count] = Some(val);
            target_count += 1;
        }
    }

    if target_count == 0 {
        targets[0] = Some(".");
        target_count = 1;
    }

    let mut had_error = false;
    for (idx, &target_opt) in targets[..target_count].iter().enumerate() {
        let target = match target_opt {
            Some(t) => t,
            None => continue,
        };

        if target_count > 1 {
            if idx > 0 {
                write(STDOUT_FILENO, b"\n");
            }
            write(STDOUT_FILENO, target.as_bytes());
            write(STDOUT_FILENO, b":\n");
        }

        if !list_directory(target, show_all, long_format) {
            had_error = true;
            print_str(STDERR_FILENO, t("ls: невозможно получить доступ к '", "ls: cannot access '"));
            print_str(STDERR_FILENO, target);
            print_str(STDERR_FILENO, t("': Нет такого файла или каталога\n", "': No such file or directory\n"));
        }
    }

    if had_error {
        exit(1);
    } else {
        exit(0);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in ls\n");
    exit(1);
}
