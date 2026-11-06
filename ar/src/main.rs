#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod header;

use libwunix::{
    chmod, close, exit, lseek, lstat, open, read, str_utils, t, write, O_CREAT,
    O_RDONLY, O_TRUNC, O_WRONLY, SEEK_END, SEEK_SET, Stat, STDERR_FILENO,
    STDOUT_FILENO,
};
use header::{ArHeader, AR_MAGIC, HEADER_SIZE};

enum ArCommand {
    QuickAppend,
    List,
    Extract,
}

struct ArArgs<'a> {
    command: ArCommand,
    verbose: bool,
    archive_file: Option<&'a [u8]>,
    files: [&'a [u8]; 32],
    file_count: usize,
}

impl<'a> Default for ArArgs<'a> {
    fn default() -> Self {
        Self {
            command: ArCommand::List,
            verbose: false,
            archive_file: None,
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
            "Использование: ar [КЛЮЧ] АРХИВ [ФАЙЛ]...\n\
             Архиватор Unix Common Archive (ar) для статических библиотек .a и .deb.\n\n\
             Команды:\n\
               r, rcs   добавить/заменить файлы в архиве\n\
               t        показать список файлов в архиве\n\
               x        извлечь файлы из архива\n\n\
             Модификаторы:\n\
               v        подробный вывод\n\
               --help   показать эту справку\n",
            "Usage: ar [OPTION] ARCHIVE [FILE]...\n\
             Unix Common Archive (ar) utility for .a static libraries and .deb.\n\n\
             Commands:\n\
               r, rcs   insert files into archive\n\
               t        display archive contents table\n\
               x        extract members from archive\n\n\
             Modifiers:\n\
               v        verbose\n\
               --help   display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> ArArgs<'a> {
    let mut args = ArArgs::default();
    let mut i = 1;

    if argc > 1 {
        let cmd_arg = parse_cstr(unsafe { *argv.add(1) });
        if cmd_arg == b"--help" {
            print_help();
            exit(0);
        }

        for &b in cmd_arg {
            match b {
                b'r' | b'q' => args.command = ArCommand::QuickAppend,
                b't' => args.command = ArCommand::List,
                b'x' => args.command = ArCommand::Extract,
                b'v' => args.verbose = true,
                _ => {}
            }
        }
        i = 2;
    }

    if i < argc {
        args.archive_file = Some(parse_cstr(unsafe { *argv.add(i as usize) }));
        i += 1;
    }

    while i < argc {
        let f = parse_cstr(unsafe { *argv.add(i as usize) });
        if args.file_count < args.files.len() {
            args.files[args.file_count] = f;
            args.file_count += 1;
        }
        i += 1;
    }

    args
}

fn run_ar(args: ArArgs) -> u64 {
    let archive_path = match args.archive_file {
        Some(p) => p,
        None => {
            print_help();
            return 1;
        }
    };

    let mut path_nt = [0u8; 513];
    let len = archive_path.len().min(512);
    path_nt[..len].copy_from_slice(&archive_path[..len]);
    path_nt[len] = 0;

    match args.command {
        ArCommand::QuickAppend => {
            let out_fd_res = open(&path_nt[..len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, 0o644);
            if out_fd_res < 0 {
                return 1;
            }
            let out_fd = out_fd_res as u64;

            // Записываем магическую сигнатуру !<arch>\n
            write(out_fd, &AR_MAGIC);

            for f_idx in 0..args.file_count {
                let fpath = args.files[f_idx];
                let mut f_nt = [0u8; 513];
                let f_len = fpath.len().min(512);
                f_nt[..f_len].copy_from_slice(&fpath[..f_len]);
                f_nt[f_len] = 0;

                let in_fd_res = open(&f_nt[..f_len + 1], O_RDONLY as u64, 0);
                if in_fd_res < 0 {
                    continue;
                }
                let in_fd = in_fd_res as u64;

                let mut st = Stat {
                    st_dev: 0, st_ino: 0, st_nlink: 0, st_mode: 0, st_uid: 0, st_gid: 0,
                    __pad0: 0, st_rdev: 0, st_size: 0, st_blksize: 0, st_blocks: 0,
                    st_atime: 0, st_atime_nsec: 0, st_mtime: 0, st_mtime_nsec: 0,
                    st_ctime: 0, st_ctime_nsec: 0, __unused: [0; 3],
                };
                let _ = lstat(&f_nt[..f_len + 1], &mut st);

                // Имя файла в формате System V (с завершающим слешем /)
                let base_name = match fpath.iter().rposition(|&b| b == b'/') {
                    Some(idx) => &fpath[idx + 1..],
                    None => fpath,
                };

                let mut hdr = ArHeader::default();
                let b_len = base_name.len().min(15);
                hdr.name[..b_len].copy_from_slice(&base_name[..b_len]);
                hdr.name[b_len] = b'/';
                hdr.mtime = st.st_mtime as u64;
                hdr.uid = st.st_uid;
                hdr.gid = st.st_gid;
                hdr.mode = (st.st_mode & 0o7777) as u32;
                hdr.size = st.st_size as u64;

                let mut hdr_buf = [0u8; HEADER_SIZE];
                hdr.serialize(&mut hdr_buf);
                write(out_fd, &hdr_buf);

                // Копируем данные
                let mut buf = [0u8; 4096];
                let mut remaining = hdr.size;
                while remaining > 0 {
                    let to_read = remaining.min(buf.len() as u64) as usize;
                    let rd = read(in_fd, &mut buf[..to_read]);
                    if rd == 0 {
                        break;
                    }
                    write(out_fd, &buf[..rd as usize]);
                    remaining -= rd;
                }
                close(in_fd);

                // Выравнивание до четного смещения
                if (hdr.size & 1) != 0 {
                    write(out_fd, b"\n");
                }

                if args.verbose {
                    write(STDOUT_FILENO, b"a - ");
                    write(STDOUT_FILENO, fpath);
                    write(STDOUT_FILENO, b"\n");
                }
            }
            close(out_fd);
            0
        }
        ArCommand::List | ArCommand::Extract => {
            let in_fd_res = open(&path_nt[..len + 1], O_RDONLY as u64, 0);
            if in_fd_res < 0 {
                return 1;
            }
            let in_fd = in_fd_res as u64;

            let file_size = lseek(in_fd, 0, SEEK_END as i32) as u64;
            lseek(in_fd, 0, SEEK_SET as i32);

            let mut magic = [0u8; 8];
            if read(in_fd, &mut magic) < 8 || magic != AR_MAGIC {
                write(STDERR_FILENO, b"ar: not an archive\n");
                close(in_fd);
                return 1;
            }

            let mut offset = 8u64;
            while offset + HEADER_SIZE as u64 <= file_size {
                lseek(in_fd, offset as i64, SEEK_SET as i32);
                let mut hdr_buf = [0u8; HEADER_SIZE];
                if read(in_fd, &mut hdr_buf) < HEADER_SIZE as u64 {
                    break;
                }

                let hdr = match ArHeader::parse(&hdr_buf) {
                    Some(h) => h,
                    None => break,
                };

                let name_len = hdr.name.iter().position(|&b| b == b'/' || b == b' ').unwrap_or(hdr.name.len());
                let member_name = &hdr.name[..name_len];

                if matches!(args.command, ArCommand::List) {
                    write(STDOUT_FILENO, member_name);
                    write(STDOUT_FILENO, b"\n");
                } else if matches!(args.command, ArCommand::Extract) {
                    let mut out_nt = [0u8; 513];
                    let m_len = member_name.len().min(512);
                    out_nt[..m_len].copy_from_slice(&member_name[..m_len]);
                    out_nt[m_len] = 0;

                    let out_fd_res = open(&out_nt[..m_len + 1], (O_CREAT | O_WRONLY | O_TRUNC) as u64, hdr.mode as u64);
                    if out_fd_res >= 0 {
                        let out_fd = out_fd_res as u64;
                        let mut buf = [0u8; 4096];
                        let mut remaining = hdr.size;
                        while remaining > 0 {
                            let to_read = remaining.min(buf.len() as u64) as usize;
                            let rd = read(in_fd, &mut buf[..to_read]);
                            if rd == 0 {
                                break;
                            }
                            write(out_fd, &buf[..rd as usize]);
                            remaining -= rd;
                        }
                        close(out_fd);
                        chmod(&out_nt[..m_len + 1], hdr.mode);
                    }

                    if args.verbose {
                        write(STDOUT_FILENO, b"x - ");
                        write(STDOUT_FILENO, member_name);
                        write(STDOUT_FILENO, b"\n");
                    }
                }

                let padded_size = (hdr.size + 1) & !1;
                offset += HEADER_SIZE as u64 + padded_size;
            }

            close(in_fd);
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
    let code = run_ar(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in ar\n");
    exit(1);
}
