#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod create;
mod extract;
mod header;
mod list;

use libwunix::{
    chdir, close, exit, open, read, str_utils, t, write, O_CREAT, O_RDONLY, O_TRUNC,
    O_WRONLY, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO,
};
use header::{TarEntryHeader, BLOCK_SIZE};

struct TarArgs<'a> {
    create: bool,
    extract: bool,
    list: bool,
    verbose: bool,
    gzip: bool,
    archive_file: Option<&'a [u8]>,
    change_dir: Option<&'a [u8]>,
    files: [&'a [u8]; 32],
    file_count: usize,
}

impl<'a> Default for TarArgs<'a> {
    fn default() -> Self {
        Self {
            create: false,
            extract: false,
            list: false,
            verbose: false,
            gzip: false,
            archive_file: None,
            change_dir: None,
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
            "Использование: tar [КЛЮЧ]... [ФАЙЛ]...\n\
             Архиватор POSIX ustar (tar).\n\n\
             Основные режимы работы:\n\
               -c, --create        создать новый архив\n\
               -x, --extract       извлечь файлы из архива\n\
               -t, --list          показать содержимое архива\n\n\
             Параметры:\n\
               -f, --file=АРХИВ    имя файла архива (или '-' для stdin/stdout)\n\
               -v, --verbose       подробный вывод обрабатываемых файлов\n\
               -z, --gzip          сжатие/распаковка через gzip\n\
               -C, --directory=КАТ перейти в каталог перед началом работы\n\
                   --help          показать эту справку\n",
            "Usage: tar [OPTION]... [FILE]...\n\
             POSIX ustar archive utility.\n\n\
             Main operation modes:\n\
               -c, --create        create a new archive\n\
               -x, --extract       extract files from an archive\n\
               -t, --list          list the contents of an archive\n\n\
             Operation modifiers:\n\
               -f, --file=ARCHIVE  use archive file (or '-' for stdin/stdout)\n\
               -v, --verbose       verbosely list files processed\n\
               -z, --gzip          filter archive through gzip\n\
               -C, --directory=DIR change to directory DIR\n\
                   --help          display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> TarArgs<'a> {
    let mut args = TarArgs::default();
    let mut i = 1;

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"-c" || arg == b"--create" {
            args.create = true;
        } else if arg == b"-x" || arg == b"--extract" {
            args.extract = true;
        } else if arg == b"-t" || arg == b"--list" {
            args.list = true;
        } else if arg == b"-v" || arg == b"--verbose" {
            args.verbose = true;
        } else if arg == b"-z" || arg == b"--gzip" {
            args.gzip = true;
        } else if arg == b"-f" {
            i += 1;
            if i < argc {
                args.archive_file = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"--file=") {
            args.archive_file = Some(&arg[7..]);
        } else if arg == b"-C" || arg == b"--directory" {
            i += 1;
            if i < argc {
                args.change_dir = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"--directory=") {
            args.change_dir = Some(&arg[12..]);
        } else if arg.starts_with(b"-") && arg.len() > 1 {
            // Поддержка сгруппированных ключей, например: -cvf, -xvf, -tvf
            let mut j = 1;
            while j < arg.len() {
                match arg[j] {
                    b'c' => args.create = true,
                    b'x' => args.extract = true,
                    b't' => args.list = true,
                    b'v' => args.verbose = true,
                    b'z' => args.gzip = true,
                    b'f' => {
                        if j + 1 < arg.len() {
                            args.archive_file = Some(&arg[j + 1..]);
                            break;
                        } else {
                            i += 1;
                            if i < argc {
                                args.archive_file = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
                            }
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        } else if !arg.starts_with(b"-") && (args.create || args.extract || args.list) {
            if args.file_count < args.files.len() {
                args.files[args.file_count] = arg;
                args.file_count += 1;
            }
        } else if !arg.starts_with(b"-") && i == 1 {
            // Традиционный синтаксис tar без минуса: tar cvf archive.tar file
            let mut j = 0;
            while j < arg.len() {
                match arg[j] {
                    b'c' => args.create = true,
                    b'x' => args.extract = true,
                    b't' => args.list = true,
                    b'v' => args.verbose = true,
                    b'z' => args.gzip = true,
                    b'f' => {
                        i += 1;
                        if i < argc {
                            args.archive_file = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        i += 1;
    }

    args
}

fn open_archive_read(file: Option<&[u8]>) -> i64 {
    match file {
        None | Some(b"-") => STDIN_FILENO as i64,
        Some(path) => {
            let mut path_nt = [0u8; 513];
            let len = path.len().min(512);
            path_nt[..len].copy_from_slice(&path[..len]);
            path_nt[len] = 0;
            open(&path_nt[..len + 1], O_RDONLY as u64, 0)
        }
    }
}

fn open_archive_write(file: Option<&[u8]>) -> i64 {
    match file {
        None | Some(b"-") => STDOUT_FILENO as i64,
        Some(path) => {
            let mut path_nt = [0u8; 513];
            let len = path.len().min(512);
            path_nt[..len].copy_from_slice(&path[..len]);
            path_nt[len] = 0;
            open(&path_nt[..len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644)
        }
    }
}

fn run_tar(args: TarArgs) -> i32 {
    if let Some(cd) = args.change_dir {
        let mut cd_nt = [0u8; 513];
        let len = cd.len().min(512);
        cd_nt[..len].copy_from_slice(&cd[..len]);
        cd_nt[len] = 0;
        chdir(&cd_nt[..len + 1]);
    }

    if args.create {
        let out_fd = open_archive_write(args.archive_file);
        if out_fd < 0 {
            write(STDERR_FILENO, b"tar: ");
            write(
                STDERR_FILENO,
                t("не удалось открыть архив для записи\n", "failed to open archive for writing\n").as_bytes(),
            );
            return 1;
        }

        for i in 0..args.file_count {
            let path = args.files[i];
            let _ = create::add_path(out_fd as u64, path, args.verbose);
        }

        let _ = create::finalize_archive(out_fd as u64);
        if out_fd as u64 != STDOUT_FILENO {
            close(out_fd as u64);
        }
        return 0;
    }

    if args.list || args.extract {
        let in_fd = open_archive_read(args.archive_file);
        if in_fd < 0 {
            write(STDERR_FILENO, b"tar: ");
            write(
                STDERR_FILENO,
                t("не удалось открыть архив для чтения\n", "failed to open archive for reading\n").as_bytes(),
            );
            return 1;
        }

        let mut block = [0u8; BLOCK_SIZE];
        loop {
            let rd = read(in_fd as u64, &mut block);
            if rd < BLOCK_SIZE as u64 {
                break;
            }

            match TarEntryHeader::parse(&block) {
                Some(header) => {
                    if args.list {
                        list::list_entry(&header, args.verbose);
                        let _ = extract::skip_file_data(in_fd as u64, header.size);
                    } else if args.extract {
                        let _ = extract::extract_entry(in_fd as u64, &header, args.verbose);
                    }
                }
                None => {
                    // Конец архива (нулевой блок)
                    break;
                }
            }
        }

        if in_fd as u64 != STDIN_FILENO {
            close(in_fd as u64);
        }
        return 0;
    }

    print_help();
    1
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }
    let args = parse_cli_args(argc, argv);
    let code = run_tar(args);
    exit(code as u64);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in tar\n");
    exit(1);
}
