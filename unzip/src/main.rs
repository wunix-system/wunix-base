#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod extract;
mod format;
mod list;

use libwunix::{
    close, exit, lseek, open, read, str_utils, t, write, O_RDONLY, SEEK_END, SEEK_SET,
    STDERR_FILENO, STDOUT_FILENO,
};
use format::{find_eocd, parse_central_dir_entry, CENTRAL_HEADER_SIZE};

struct UnzipArgs<'a> {
    archive_file: Option<&'a [u8]>,
    dest_dir: Option<&'a [u8]>,
    list: bool,
    verbose: bool,
    test_only: bool,
    overwrite: bool,
    quiet: bool,
}

impl<'a> Default for UnzipArgs<'a> {
    fn default() -> Self {
        Self {
            archive_file: None,
            dest_dir: None,
            list: false,
            verbose: false,
            test_only: false,
            overwrite: false,
            quiet: false,
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
            "Использование: unzip [-l] [-v] [-t] [-d КАТАЛОГ] [-o] [-q] архив.zip\n\
             Утилита распаковки архивов ZIP для Wunix OS.\n\n\
             Основные режимы:\n\
               -l           показать список файлов в архиве\n\
               -v           подробный список файлов (метод, размер, CRC32)\n\
               -t           проверить целостность архива (проверка CRC32)\n\n\
             Параметры распаковки:\n\
               -d КАТАЛОГ   директория для извлечения файлов\n\
               -o           перезаписывать существующие файлы без подтверждения\n\
               -q           тихий режим (минимальный вывод)\n\
                   --help   показать эту справку\n",
            "Usage: unzip [-l] [-v] [-t] [-d EXDIR] [-o] [-q] archive.zip\n\
             ZIP archive extraction utility for Wunix OS.\n\n\
             Main modes:\n\
               -l           list archive files (short format)\n\
               -v           list archive files (verbose format)\n\
               -t           test archive files integrity (CRC32 check)\n\n\
             Modifiers:\n\
               -d EXDIR     extract files into EXDIR\n\
               -o           overwrite files WITHOUT prompting\n\
               -q           quiet mode\n\
                   --help   display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> UnzipArgs<'a> {
    let mut args = UnzipArgs::default();
    let mut i = 1;

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"-l" {
            args.list = true;
        } else if arg == b"-v" {
            args.verbose = true;
        } else if arg == b"-t" {
            args.test_only = true;
        } else if arg == b"-o" {
            args.overwrite = true;
        } else if arg == b"-q" {
            args.quiet = true;
        } else if arg == b"-d" {
            i += 1;
            if i < argc {
                args.dest_dir = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"-d") {
            args.dest_dir = Some(&arg[2..]);
        } else if arg.starts_with(b"-") && arg.len() > 1 {
            for &b in &arg[1..] {
                match b {
                    b'l' => args.list = true,
                    b'v' => args.verbose = true,
                    b't' => args.test_only = true,
                    b'o' => args.overwrite = true,
                    b'q' => args.quiet = true,
                    _ => {}
                }
            }
        } else if args.archive_file.is_none() {
            args.archive_file = Some(arg);
        }
        i += 1;
    }

    args
}

fn run_unzip(args: UnzipArgs) -> u64 {
    let archive_path = match args.archive_file {
        Some(path) => path,
        None => {
            print_help();
            return 1;
        }
    };

    let mut path_nt = [0u8; 513];
    let len = archive_path.len().min(512);
    path_nt[..len].copy_from_slice(&archive_path[..len]);
    path_nt[len] = 0;

    let fd_res = open(&path_nt[..len + 1], O_RDONLY as u64, 0);
    if fd_res < 0 {
        write(STDERR_FILENO, b"unzip: ");
        write(
            STDERR_FILENO,
            t("не удалось открыть архив: ", "failed to open archive: ").as_bytes(),
        );
        write(STDERR_FILENO, archive_path);
        write(STDERR_FILENO, b"\n");
        return 1;
    }
    let fd = fd_res as u64;

    let file_size_res = lseek(fd, 0, SEEK_END as i32);
    if file_size_res < 0 {
        close(fd);
        return 1;
    }
    let file_size = file_size_res as u64;

    let eocd = match find_eocd(fd, file_size) {
        Ok(info) => info,
        Err(_) => {
            write(STDERR_FILENO, b"unzip: ");
            write(
                STDERR_FILENO,
                t("архив повреждён или не является файлом ZIP\n", "archive corrupted or not a valid ZIP file\n").as_bytes(),
            );
            close(fd);
            return 1;
        }
    };

    if args.list || args.verbose {
        list::print_list_header(args.verbose);
    } else if !args.quiet {
        write(STDOUT_FILENO, b"Archive:  ");
        write(STDOUT_FILENO, archive_path);
        write(STDOUT_FILENO, b"\n");
    }

    let mut total_uncompressed = 0u64;
    let mut total_compressed = 0u64;
    let mut files_processed = 0usize;

    let mut cd_cur_offset = eocd.cd_offset;

    for _ in 0..eocd.total_entries {
        lseek(fd, cd_cur_offset as i64, SEEK_SET as i32);

        let mut cd_buf = [0u8; CENTRAL_HEADER_SIZE];
        if read(fd, &mut cd_buf) < CENTRAL_HEADER_SIZE as u64 {
            break;
        }

        let entry = match parse_central_dir_entry(&cd_buf) {
            Some(e) => e,
            None => break,
        };

        let mut name_buf = [0u8; 512];
        let n_to_read = (entry.name_len as usize).min(name_buf.len());
        let n_read = read(fd, &mut name_buf[..n_to_read]);
        let name_slice = &name_buf[..n_read as usize];

        if args.list || args.verbose {
            list::print_entry_line(&entry, name_slice, args.verbose);
        } else {
            let is_verbose = !args.quiet;
            let _ = extract::extract_or_test_entry(
                fd,
                &entry,
                name_slice,
                args.dest_dir,
                args.test_only,
                args.overwrite,
                is_verbose,
            );
        }

        total_uncompressed += entry.uncompressed_size as u64;
        total_compressed += entry.compressed_size as u64;
        files_processed += 1;

        cd_cur_offset += (CENTRAL_HEADER_SIZE + entry.name_len as usize + entry.extra_len as usize + entry.comment_len as usize) as u32;
    }

    if args.list || args.verbose {
        list::print_list_footer(files_processed, total_uncompressed, total_compressed, args.verbose);
    } else if args.test_only && !args.quiet {
        write(STDOUT_FILENO, b"No errors detected in compressed data of ");
        write(STDOUT_FILENO, archive_path);
        write(STDOUT_FILENO, b".\n");
    }

    close(fd);
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
    let code = run_unzip(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in unzip\n");
    exit(1);
}
