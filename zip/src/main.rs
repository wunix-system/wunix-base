#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod create;
mod format;

use libwunix::{
    close, exit, open, str_utils, t, write, O_CREAT, O_TRUNC, O_WRONLY,
    STDERR_FILENO, STDOUT_FILENO,
};
use create::{add_zip_file, finalize_zip, ZipArchiveState};

struct ZipArgs<'a> {
    archive_file: Option<&'a [u8]>,
    recursive: bool,
    store_only: bool,
    verbose: bool,
    quiet: bool,
    files: [&'a [u8]; 32],
    file_count: usize,
}

impl<'a> Default for ZipArgs<'a> {
    fn default() -> Self {
        Self {
            archive_file: None,
            recursive: false,
            store_only: false,
            verbose: false,
            quiet: false,
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
            "Использование: zip [-r] [-0] [-v] [-q] архив.zip файл1 [файл2...]\n\
             Архиватор ZIP для Wunix OS (PKWARE standard).\n\n\
             Параметры:\n\
               -r, --recurse-paths  рекурсивно добавлять подкаталоги\n\
               -0, --store          сохранять без сжатия (метод Store)\n\
               -v, --verbose        подробный вывод\n\
               -q, --quiet          тихий режим (без вывода в stdout)\n\
                   --help           показать эту справку\n",
            "Usage: zip [-r] [-0] [-v] [-q] archive.zip file1 [file2...]\n\
             ZIP archiver for Wunix OS (PKWARE standard).\n\n\
             Options:\n\
               -r, --recurse-paths  recurse into directories\n\
               -0, --store          store only without compression\n\
               -v, --verbose        verbose output\n\
               -q, --quiet          quiet mode\n\
                   --help           display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> ZipArgs<'a> {
    let mut args = ZipArgs::default();
    let mut i = 1;

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"-r" || arg == b"--recurse-paths" {
            args.recursive = true;
        } else if arg == b"-0" || arg == b"--store" {
            args.store_only = true;
        } else if arg == b"-v" || arg == b"--verbose" {
            args.verbose = true;
        } else if arg == b"-q" || arg == b"--quiet" {
            args.quiet = true;
        } else if arg.starts_with(b"-") && arg.len() > 1 {
            for &b in &arg[1..] {
                match b {
                    b'r' => args.recursive = true,
                    b'0' => args.store_only = true,
                    b'v' => args.verbose = true,
                    b'q' => args.quiet = true,
                    _ => {}
                }
            }
        } else if args.archive_file.is_none() {
            args.archive_file = Some(arg);
        } else if args.file_count < args.files.len() {
            args.files[args.file_count] = arg;
            args.file_count += 1;
        }
        i += 1;
    }

    args
}

fn run_zip(args: ZipArgs) -> u64 {
    let archive_path = match args.archive_file {
        Some(path) => path,
        None => {
            print_help();
            return 1;
        }
    };

    if args.file_count == 0 {
        write(STDERR_FILENO, b"zip: ");
        write(
            STDERR_FILENO,
            t("не указаны входные файлы для архивации\n", "no input files specified for archiving\n").as_bytes(),
        );
        return 1;
    }

    let mut path_nt = [0u8; 513];
    let len = archive_path.len().min(512);
    path_nt[..len].copy_from_slice(&archive_path[..len]);
    path_nt[len] = 0;

    let out_fd = open(&path_nt[..len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
    if out_fd < 0 {
        write(STDERR_FILENO, b"zip: ");
        write(
            STDERR_FILENO,
            t("не удалось создать архивный файл: ", "failed to create archive file: ").as_bytes(),
        );
        write(STDERR_FILENO, archive_path);
        write(STDERR_FILENO, b"\n");
        return 1;
    }

    let mut state = ZipArchiveState::new();
    let is_verbose = !args.quiet && (args.verbose || true);

    for i in 0..args.file_count {
        let fpath = args.files[i];
        if let Err(_) = add_zip_file(out_fd as u64, &mut state, fpath, args.store_only, is_verbose) {
            write(STDERR_FILENO, b"zip: ");
            write(
                STDERR_FILENO,
                t("ошибка добавления файла: ", "error adding file: ").as_bytes(),
            );
            write(STDERR_FILENO, fpath);
            write(STDERR_FILENO, b"\n");
        }
    }

    let _ = finalize_zip(out_fd as u64, &state);
    close(out_fd as u64);

    0
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }
    let args = parse_cli_args(argc, argv);
    let code = run_zip(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in zip\n");
    exit(1);
}
