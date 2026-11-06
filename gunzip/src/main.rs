#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

use libwunix::{
    close, exit, lseek, open, read, str_utils, t, unlink, write, O_CREAT, O_RDONLY,
    O_TRUNC, O_WRONLY, SEEK_END, SEEK_SET, STDERR_FILENO, STDOUT_FILENO,
};

struct GunzipArgs<'a> {
    to_stdout: bool,
    keep: bool,
    force: bool,
    verbose: bool,
    test: bool,
    files: [&'a [u8]; 32],
    file_count: usize,
}

impl<'a> Default for GunzipArgs<'a> {
    fn default() -> Self {
        Self {
            to_stdout: false,
            keep: false,
            force: false,
            verbose: false,
            test: false,
            files: [&[]; 32],
            file_count: 0,
        }
    }
}

fn parse_cstr<'a>(ptr: *const u8) -> &'a [u8] {
    if ptr.is_null() {
        return &[];
    }
    let len = str_utils::strlen(ptr);
    unsafe { core::slice::from_raw_parts(ptr, len) }
}

fn print_help() {
    write(
        STDOUT_FILENO,
        t(
            "Использование: gunzip [КЛЮЧ]... [ФАЙЛ]...\n\
             Распаковка файлов формата GZIP (.gz) для Wunix OS.\n\n\
             Параметры:\n\
               -c, --stdout   выводить результат в стандартный вывод (STDOUT)\n\
               -f, --force    перезаписывать существующие файлы\n\
               -k, --keep     сохранять исходный .gz файл (не удалять)\n\
               -t, --test     проверить целостность сжатого файла\n\
               -v, --verbose  подробный вывод\n\
                   --help     показать эту справку\n",
            "Usage: gunzip [OPTION]... [FILE]...\n\
             Decompress files from GZIP format (.gz) for Wunix OS.\n\n\
             Options:\n\
               -c, --stdout   write on standard output\n\
               -f, --force    force overwrite of output file\n\
               -k, --keep     keep (don't delete) input files\n\
               -t, --test     test compressed file integrity\n\
               -v, --verbose  verbose mode\n\
                   --help     display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> GunzipArgs<'a> {
    let mut args = GunzipArgs::default();
    let mut i = 1;

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"-c" || arg == b"--stdout" {
            args.to_stdout = true;
        } else if arg == b"-k" || arg == b"--keep" {
            args.keep = true;
        } else if arg == b"-f" || arg == b"--force" {
            args.force = true;
        } else if arg == b"-v" || arg == b"--verbose" {
            args.verbose = true;
        } else if arg == b"-t" || arg == b"--test" {
            args.test = true;
        } else if arg.starts_with(b"-") && arg.len() > 1 {
            for &b in &arg[1..] {
                match b {
                    b'c' => args.to_stdout = true,
                    b'k' => args.keep = true,
                    b'f' => args.force = true,
                    b'v' => args.verbose = true,
                    b't' => args.test = true,
                    _ => {}
                }
            }
        } else if args.file_count < args.files.len() {
            args.files[args.file_count] = arg;
            args.file_count += 1;
        }
        i += 1;
    }

    args
}

fn decompress_file(file_path: &[u8], args: &GunzipArgs) -> Result<(), ()> {
    let mut in_path_nt = [0u8; 513];
    let len = file_path.len().min(512);
    in_path_nt[..len].copy_from_slice(&file_path[..len]);
    in_path_nt[len] = 0;

    let in_fd_res = open(&in_path_nt[..len + 1], O_RDONLY as u64, 0);
    if in_fd_res < 0 {
        write(STDERR_FILENO, b"gunzip: cannot open ");
        write(STDERR_FILENO, file_path);
        write(STDERR_FILENO, b"\n");
        return Err(());
    }
    let in_fd = in_fd_res as u64;

    let file_size_res = lseek(in_fd, 0, SEEK_END as i32);
    lseek(in_fd, 0, SEEK_SET as i32);
    if file_size_res < 0 {
        close(in_fd);
        return Err(());
    }
    let file_size = file_size_res as usize;

    let mut input_buf = [0u8; 65536];
    let to_read = file_size.min(input_buf.len());
    let rd = read(in_fd, &mut input_buf[..to_read]);
    close(in_fd);
    if rd == 0 && file_size > 0 {
        return Err(());
    }
    let data = &input_buf[..rd as usize];

    let out_fd = if args.test {
        None
    } else if args.to_stdout {
        Some(STDOUT_FILENO)
    } else {
        let out_name = if file_path.ends_with(b".gz") {
            &file_path[..file_path.len() - 3]
        } else {
            file_path
        };
        let mut out_nt = [0u8; 513];
        let o_len = out_name.len().min(512);
        out_nt[..o_len].copy_from_slice(&out_name[..o_len]);
        out_nt[o_len] = 0;
        let fd = open(&out_nt[..o_len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
        if fd < 0 {
            return Err(());
        }
        Some(fd as u64)
    };

    let res = libwunix::gzip_decompress(data, |chunk| {
        if let Some(fd) = out_fd {
            write(fd, chunk);
        }
        Ok(())
    });

    if let Some(fd) = out_fd {
        if fd != STDOUT_FILENO {
            close(fd);
        }
    }

    if res.is_err() {
        write(STDERR_FILENO, b"gunzip: corrupt input or CRC mismatch in ");
        write(STDERR_FILENO, file_path);
        write(STDERR_FILENO, b"\n");
        return Err(());
    }

    if !args.keep && !args.to_stdout && !args.test {
        unlink(&in_path_nt[..len + 1]);
    }

    Ok(())
}

fn run_gunzip(args: GunzipArgs) -> u64 {
    if args.file_count == 0 {
        write(STDERR_FILENO, b"gunzip: no input files\n");
        return 1;
    }

    let mut exit_code = 0u64;
    for i in 0..args.file_count {
        if decompress_file(args.files[i], &args).is_err() {
            exit_code = 1;
        }
    }

    exit_code
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }
    let args = parse_cli_args(argc, argv);
    let code = run_gunzip(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in gunzip\n");
    exit(1);
}
