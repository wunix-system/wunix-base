#![no_std]
#![no_main]

use libwunix::{
    exit, getcwd, readlink, str_utils, t, write, STDERR_FILENO, STDOUT_FILENO,
};

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
            "Использование: readlink [КЛЮЧ]... ФАЙЛ...\n\
             Печатает значение символической ссылки или каноническое имя файла.\n\n\
               -f, --canonicalize      канонизировать путь, разрешая все символические ссылки\n  \
               -n, --no-newline        не выводить завершающий перевод строки\n  \
               -q, -s, --silent, --quiet  подавлять большинство сообщений об ошибках\n  \
               -v, --verbose           сообщать обо всех ошибках\n  \
               -h, --help              показать эту справку и выйти\n",
            "Usage: readlink [OPTION]... FILE...\n\
             Print value of a symbolic link or canonical file name.\n\n\
               -f, --canonicalize      canonicalize by following every symlink\n  \
               -n, --no-newline        do not output the trailing delimiter\n  \
               -q, -s, --silent, --quiet  suppress most error messages\n  \
               -v, --verbose           report error messages\n  \
               -h, --help              display this help and exit\n",
        ),
    );
}

// Нормализует путь, устраняя '.' и '..'
fn normalize_path(path: &str, out: &mut [u8]) -> usize {
    let mut parts: [&str; 64] = [""; 64];
    let mut count = 0;

    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if count > 0 {
                count -= 1;
            }
        } else if count < parts.len() {
            parts[count] = seg;
            count += 1;
        }
    }

    let is_absolute = path.starts_with('/');
    let mut len = 0;

    if is_absolute {
        if len < out.len() {
            out[len] = b'/';
            len += 1;
        }
    }

    for (i, seg) in parts[..count].iter().enumerate() {
        if i > 0 || !is_absolute {
            if i > 0 && len < out.len() {
                out[len] = b'/';
                len += 1;
            }
        }
        let b = seg.as_bytes();
        let copy_len = b.len().min(out.len().saturating_sub(len));
        out[len..len + copy_len].copy_from_slice(&b[..copy_len]);
        len += copy_len;
    }

    if len == 0 {
        if is_absolute && len < out.len() {
            out[0] = b'/';
            len = 1;
        } else if len < out.len() {
            out[0] = b'.';
            len = 1;
        }
    }

    len
}

// Канонизирует путь, разрешая промежуточные симлинки
fn canonicalize(path: &str, out: &mut [u8]) -> Result<usize, i64> {
    let mut current_buf = [0u8; 1024];
    let mut curr_len = 0;

    if !path.starts_with('/') {
        let mut cwd_buf = [0u8; 512];
        let cwd_ret = getcwd(&mut cwd_buf);
        if cwd_ret > 0 {
            let cwd_len = cwd_ret as usize;
            current_buf[..cwd_len].copy_from_slice(&cwd_buf[..cwd_len]);
            curr_len = cwd_len;
            if !current_buf[..curr_len].ends_with(b"/") && curr_len < current_buf.len() {
                current_buf[curr_len] = b'/';
                curr_len += 1;
            }
        } else {
            current_buf[0] = b'/';
            curr_len = 1;
        }
    }

    let p_bytes = path.as_bytes();
    let copy_p = p_bytes.len().min(current_buf.len().saturating_sub(curr_len));
    current_buf[curr_len..curr_len + copy_p].copy_from_slice(&p_bytes[..copy_p]);
    curr_len += copy_p;

    let mut norm_buf = [0u8; 1024];
    let norm_len = normalize_path(
        core::str::from_utf8(&current_buf[..curr_len]).unwrap_or("/"),
        &mut norm_buf,
    );

    // Разрешаем симлинки (максимум 32 итерации для предотвращения зацикливания)
    let mut cur_path_buf = [0u8; 1024];
    cur_path_buf[..norm_len].copy_from_slice(&norm_buf[..norm_len]);
    let mut cur_path_len = norm_len;

    for _ in 0..32 {
        let mut link_target = [0u8; 1024];
        let mut null_terminated = [0u8; 1025];
        null_terminated[..cur_path_len].copy_from_slice(&cur_path_buf[..cur_path_len]);
        null_terminated[cur_path_len] = 0;

        let ret = readlink(&null_terminated[..cur_path_len + 1], &mut link_target);
        if ret <= 0 {
            // Либо не симлинк, либо ошибка - останавливаем раскрытие
            break;
        }

        let target_len = ret as usize;
        if link_target.starts_with(b"/") {
            let n_len = normalize_path(
                core::str::from_utf8(&link_target[..target_len]).unwrap_or("/"),
                &mut norm_buf,
            );
            cur_path_buf[..n_len].copy_from_slice(&norm_buf[..n_len]);
            cur_path_len = n_len;
        } else {
            // Относительная ссылка: ищем базовый каталог
            let cur_str = core::str::from_utf8(&cur_path_buf[..cur_path_len]).unwrap_or("");
            let base_dir = match cur_str.rfind('/') {
                Some(idx) => &cur_str[..idx],
                None => "",
            };

            let mut combined = [0u8; 1024];
            let mut comb_len = 0;
            let b_bytes = base_dir.as_bytes();
            combined[..b_bytes.len()].copy_from_slice(b_bytes);
            comb_len += b_bytes.len();
            if comb_len < combined.len() {
                combined[comb_len] = b'/';
                comb_len += 1;
            }
            let c_len = target_len.min(combined.len().saturating_sub(comb_len));
            combined[comb_len..comb_len + c_len].copy_from_slice(&link_target[..c_len]);
            comb_len += c_len;

            let n_len = normalize_path(
                core::str::from_utf8(&combined[..comb_len]).unwrap_or("/"),
                &mut norm_buf,
            );
            cur_path_buf[..n_len].copy_from_slice(&norm_buf[..n_len]);
            cur_path_len = n_len;
        }
    }

    let final_len = cur_path_len.min(out.len());
    out[..final_len].copy_from_slice(&cur_path_buf[..final_len]);
    Ok(final_len)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }

    let mut canonicalize_mode = false;
    let mut no_newline = false;
    let mut quiet = false;
    let mut verbose = false;
    let mut files: [Option<&str>; 16] = [None; 16];
    let mut file_count = 0;

    for i in 1..argc {
        let val = cstr_from_ptr(unsafe { *argv.add(i as usize) });
        if val == "-f" || val == "-e" || val == "-m" || val == "--canonicalize" {
            canonicalize_mode = true;
        } else if val == "-n" || val == "--no-newline" {
            no_newline = true;
        } else if val == "-q" || val == "-s" || val == "--quiet" || val == "--silent" {
            quiet = true;
        } else if val == "-v" || val == "--verbose" {
            verbose = true;
        } else if val == "-fn" || val == "-nf" {
            canonicalize_mode = true;
            no_newline = true;
        } else if val == "--help" || val == "-h" {
            print_help();
            exit(0);
        } else if !val.starts_with('-') && file_count < files.len() {
            files[file_count] = Some(val);
            file_count += 1;
        }
    }

    if file_count == 0 {
        print_str(
            STDERR_FILENO,
            t(
                "readlink: пропущен операнд\nПопробуйте 'readlink --help' для получения дополнительной информации.\n",
                "readlink: missing operand\nTry 'readlink --help' for more information.\n",
            ),
        );
        exit(1);
    }

    let mut had_error = false;

    for &file_opt in &files[..file_count] {
        let file = match file_opt {
            Some(f) => f,
            None => continue,
        };

        if canonicalize_mode {
            let mut out = [0u8; 1024];
            match canonicalize(file, &mut out) {
                Ok(len) => {
                    write(STDOUT_FILENO, &out[..len]);
                    if !no_newline {
                        write(STDOUT_FILENO, b"\n");
                    }
                }
                Err(err) => {
                    had_error = true;
                    if verbose || !quiet {
                        print_str(STDERR_FILENO, t("readlink: ошибка канонизации '", "readlink: canonicalize failed '"));
                        print_str(STDERR_FILENO, file);
                        print_str(STDERR_FILENO, "'\n");
                    }
                    let _ = err;
                }
            }
        } else {
            let mut path_null = [0u8; 1024];
            let f_bytes = file.as_bytes();
            if f_bytes.len() >= 1023 {
                had_error = true;
                continue;
            }
            path_null[..f_bytes.len()].copy_from_slice(f_bytes);
            path_null[f_bytes.len()] = 0;

            let mut target_buf = [0u8; 1024];
            let ret = readlink(&path_null[..f_bytes.len() + 1], &mut target_buf);

            if ret > 0 {
                write(STDOUT_FILENO, &target_buf[..ret as usize]);
                if !no_newline {
                    write(STDOUT_FILENO, b"\n");
                }
            } else {
                had_error = true;
                if verbose {
                    print_str(STDERR_FILENO, t("readlink: '", "readlink: '"));
                    print_str(STDERR_FILENO, file);
                    print_str(
                        STDERR_FILENO,
                        t("': Недопустимый аргумент (не является ссылкой)\n", "': Invalid argument (not a symlink)\n"),
                    );
                }
            }
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
    print_str(STDERR_FILENO, "panic in readlink\n");
    exit(1);
}
