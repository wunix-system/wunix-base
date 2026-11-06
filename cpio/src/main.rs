#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod header;

use libwunix::{
    chmod, close, exit, lstat, mkdir, open, read, str_utils, t, write, O_CREAT,
    O_RDONLY, O_TRUNC, O_WRONLY, Stat, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO,
};
use header::{CpioHeader, CPIO_HEADER_SIZE, CPIO_TRAILER};

enum CpioMode {
    Create,
    Extract,
    List,
}

struct CpioArgs<'a> {
    mode: CpioMode,
    verbose: bool,
    create_dirs: bool,
    file: Option<&'a [u8]>,
}

impl<'a> Default for CpioArgs<'a> {
    fn default() -> Self {
        Self {
            mode: CpioMode::List,
            verbose: false,
            create_dirs: false,
            file: None,
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
            "Использование: cpio {-o|--create | -i|--extract | -t|--list} [КЛЮЧИ]...\n\
             Архиватор SVR4 portable cpio (newc) для Wunix OS.\n\n\
             Основные режимы:\n\
               -o, --create    создать архив (список файлов из STDIN, вывод в STDOUT)\n\
               -i, --extract   извлечь файлы из архива (чтение из STDIN)\n\
               -t, --list      показать список содержимого архива\n\n\
             Параметры:\n\
               -v, --verbose   подробный вывод\n\
               -d, --make-directories создавать каталоги при распаковке\n\
               -F, --file=ФАЙЛ использовать указанный файл вместо STDIN/STDOUT\n\
                   --help      показать эту справку\n",
            "Usage: cpio {-o|--create | -i|--extract | -t|--list} [OPTIONS]...\n\
             SVR4 portable cpio (newc) archive utility for Wunix OS.\n\n\
             Main modes:\n\
               -o, --create    create archive (read filenames from STDIN)\n\
               -i, --extract   extract files from archive (read from STDIN)\n\
               -t, --list      list archive contents\n\n\
             Options:\n\
               -v, --verbose   verbose mode\n\
               -d, --make-directories create leading directories\n\
               -F, --file=FILE use FILE instead of STDIN/STDOUT\n\
                   --help      display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> CpioArgs<'a> {
    let mut args = CpioArgs::default();
    let mut i = 1;

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"-o" || arg == b"--create" {
            args.mode = CpioMode::Create;
        } else if arg == b"-i" || arg == b"--extract" {
            args.mode = CpioMode::Extract;
        } else if arg == b"-t" || arg == b"--list" {
            args.mode = CpioMode::List;
        } else if arg == b"-v" || arg == b"--verbose" {
            args.verbose = true;
        } else if arg == b"-d" || arg == b"--make-directories" {
            args.create_dirs = true;
        } else if arg == b"-F" {
            i += 1;
            if i < argc {
                args.file = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
            }
        } else if arg.starts_with(b"--file=") {
            args.file = Some(&arg[7..]);
        } else if arg.starts_with(b"-") && arg.len() > 1 {
            for &b in &arg[1..] {
                match b {
                    b'o' => args.mode = CpioMode::Create,
                    b'i' => args.mode = CpioMode::Extract,
                    b't' => args.mode = CpioMode::List,
                    b'v' => args.verbose = true,
                    b'd' => args.create_dirs = true,
                    _ => {}
                }
            }
        }
        i += 1;
    }

    args
}

fn ensure_parent_dirs(path: &[u8]) {
    let mut temp = [0u8; 512];
    let len = path.len().min(511);
    temp[..len].copy_from_slice(&path[..len]);

    for i in 1..len {
        if temp[i] == b'/' {
            temp[i] = 0;
            mkdir(&temp[..i + 1], 0o755);
            temp[i] = b'/';
        }
    }
}

fn skip_padding(fd: u64, size: usize) {
    let pad = (4 - (size % 4)) % 4;
    if pad > 0 {
        let mut dummy = [0u8; 4];
        read(fd, &mut dummy[..pad]);
    }
}

fn write_padding(fd: u64, size: usize) {
    let pad = (4 - (size % 4)) % 4;
    if pad > 0 {
        let zeros = [0u8; 4];
        write(fd, &zeros[..pad]);
    }
}

fn run_cpio(args: CpioArgs) -> u64 {
    let in_fd = match args.file {
        Some(f) => {
            let mut f_nt = [0u8; 513];
            let len = f.len().min(512);
            f_nt[..len].copy_from_slice(&f[..len]);
            f_nt[len] = 0;
            let fd = open(&f_nt[..len + 1], O_RDONLY as u64, 0);
            if fd < 0 {
                return 1;
            }
            fd as u64
        }
        None => STDIN_FILENO,
    };

    let out_fd = match args.file {
        Some(f) if matches!(args.mode, CpioMode::Create) => {
            let mut f_nt = [0u8; 513];
            let len = f.len().min(512);
            f_nt[..len].copy_from_slice(&f[..len]);
            f_nt[len] = 0;
            let fd = open(&f_nt[..len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
            if fd < 0 {
                return 1;
            }
            fd as u64
        }
        _ => STDOUT_FILENO,
    };

    match args.mode {
        CpioMode::Create => {
            // Читаем имена файлов построчно из STDIN
            let mut line_buf = [0u8; 512];
            let mut line_pos = 0usize;

            loop {
                let mut b = [0u8; 1];
                let rd = read(in_fd, &mut b);
                if rd == 0 {
                    break;
                }
                if b[0] == b'\n' || b[0] == 0 {
                    if line_pos > 0 {
                        let path = &line_buf[..line_pos];
                        let mut p_nt = [0u8; 513];
                        p_nt[..line_pos].copy_from_slice(path);
                        p_nt[line_pos] = 0;

                        let mut st = Stat {
                            st_dev: 0, st_ino: 0, st_nlink: 0, st_mode: 0, st_uid: 0, st_gid: 0,
                            __pad0: 0, st_rdev: 0, st_size: 0, st_blksize: 0, st_blocks: 0,
                            st_atime: 0, st_atime_nsec: 0, st_mtime: 0, st_mtime_nsec: 0,
                            st_ctime: 0, st_ctime_nsec: 0, __unused: [0; 3],
                        };
                        if lstat(&p_nt[..line_pos + 1], &mut st) == 0 {
                            let is_dir = (st.st_mode & 0o170000) == 0o040000;
                            let filesize = if is_dir { 0 } else { st.st_size as u32 };
                            let namesize = (line_pos + 1) as u32;

                            let mut hdr = CpioHeader::default();
                            hdr.ino = (st.st_ino & 0xFFFFFFFF) as u32;
                            hdr.mode = st.st_mode;
                            hdr.uid = st.st_uid;
                            hdr.gid = st.st_gid;
                            hdr.nlink = st.st_nlink as u32;
                            hdr.mtime = st.st_mtime as u32;
                            hdr.filesize = filesize;
                            hdr.namesize = namesize;

                            let mut hdr_buf = [0u8; CPIO_HEADER_SIZE];
                            hdr.serialize(&mut hdr_buf);
                            write(out_fd, &hdr_buf);

                            write(out_fd, path);
                            write(out_fd, &[0]);
                            write_padding(out_fd, CPIO_HEADER_SIZE + namesize as usize);

                            if !is_dir && filesize > 0 {
                                let f_res = open(&p_nt[..line_pos + 1], O_RDONLY as u64, 0);
                                if f_res >= 0 {
                                    let f = f_res as u64;
                                    let mut buf = [0u8; 4096];
                                    let mut rem = filesize as usize;
                                    while rem > 0 {
                                        let to_read = rem.min(buf.len());
                                        let r = read(f, &mut buf[..to_read]);
                                        if r == 0 {
                                            break;
                                        }
                                        write(out_fd, &buf[..r as usize]);
                                        rem -= r as usize;
                                    }
                                    close(f);
                                }
                                write_padding(out_fd, filesize as usize);
                            }

                            if args.verbose {
                                write(STDERR_FILENO, path);
                                write(STDERR_FILENO, b"\n");
                            }
                        }
                        line_pos = 0;
                    }
                } else if line_pos < line_buf.len() {
                    line_buf[line_pos] = b[0];
                    line_pos += 1;
                }
            }

            // Записываем финальный трейлер TRAILER!!!
            let mut tr_hdr = CpioHeader::default();
            tr_hdr.nlink = 1;
            tr_hdr.namesize = (CPIO_TRAILER.len() + 1) as u32;
            let mut tr_buf = [0u8; CPIO_HEADER_SIZE];
            tr_hdr.serialize(&mut tr_buf);
            write(out_fd, &tr_buf);
            write(out_fd, CPIO_TRAILER);
            write(out_fd, &[0]);
            write_padding(out_fd, CPIO_HEADER_SIZE + CPIO_TRAILER.len() + 1);

            0
        }
        CpioMode::List | CpioMode::Extract => {
            let mut hdr_buf = [0u8; CPIO_HEADER_SIZE];
            loop {
                let rd = read(in_fd, &mut hdr_buf);
                if rd < CPIO_HEADER_SIZE as u64 {
                    break;
                }

                let hdr = match CpioHeader::parse(&hdr_buf) {
                    Some(h) => h,
                    None => break,
                };

                let mut name_buf = [0u8; 512];
                let n_to_read = (hdr.namesize as usize).min(name_buf.len());
                let n_rd = read(in_fd, &mut name_buf[..n_to_read]);
                let name_len = (n_rd as usize).saturating_sub(1); // Исключаем завершающий нуль
                let file_name = &name_buf[..name_len];

                skip_padding(in_fd, CPIO_HEADER_SIZE + hdr.namesize as usize);

                if file_name == CPIO_TRAILER {
                    break;
                }

                if matches!(args.mode, CpioMode::List) {
                    write(STDOUT_FILENO, file_name);
                    write(STDOUT_FILENO, b"\n");

                    // Пропускаем тело файла
                    let mut dummy = [0u8; 4096];
                    let mut rem = hdr.filesize as usize;
                    while rem > 0 {
                        let to_read = rem.min(dummy.len());
                        let r = read(in_fd, &mut dummy[..to_read]);
                        if r == 0 {
                            break;
                        }
                        rem -= r as usize;
                    }
                    skip_padding(in_fd, hdr.filesize as usize);
                } else if matches!(args.mode, CpioMode::Extract) {
                    let is_dir = (hdr.mode & 0o170000) == 0o040000;
                    let mut out_nt = [0u8; 513];
                    let len = file_name.len().min(512);
                    out_nt[..len].copy_from_slice(&file_name[..len]);
                    out_nt[len] = 0;

                    if args.create_dirs {
                        ensure_parent_dirs(file_name);
                    }

                    if is_dir {
                        mkdir(&out_nt[..len + 1], hdr.mode & 0o7777);
                        skip_padding(in_fd, hdr.filesize as usize);
                    } else {
                        let out_res = open(&out_nt[..len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, (hdr.mode & 0o7777) as u64);
                        let out_f = if out_res >= 0 { Some(out_res as u64) } else { None };

                        let mut buf = [0u8; 4096];
                        let mut rem = hdr.filesize as usize;
                        while rem > 0 {
                            let to_read = rem.min(buf.len());
                            let r = read(in_fd, &mut buf[..to_read]);
                            if r == 0 {
                                break;
                            }
                            if let Some(f) = out_f {
                                write(f, &buf[..r as usize]);
                            }
                            rem -= r as usize;
                        }

                        if let Some(f) = out_f {
                            close(f);
                            chmod(&out_nt[..len + 1], hdr.mode & 0o7777);
                        }
                        skip_padding(in_fd, hdr.filesize as usize);
                    }

                    if args.verbose {
                        write(STDOUT_FILENO, file_name);
                        write(STDOUT_FILENO, b"\n");
                    }
                }
            }
            0
        }
    }
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }
    let args = parse_cli_args(argc, argv);
    let code = run_cpio(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in cpio\n");
    exit(1);
}
