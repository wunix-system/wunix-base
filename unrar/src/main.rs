#![no_std]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod extract;
mod list;
mod rar4;
mod rar5;

use libwunix::{
    close, exit, lseek, open, read, str_utils, t, write, O_RDONLY, SEEK_END,
    SEEK_SET, STDERR_FILENO, STDOUT_FILENO,
};
use rar4::{
    parse_block_header, parse_file_header, HEAD_TYPE_ENDARC, HEAD_TYPE_FILE, RAR4_SIGNATURE,
};
use rar5::{read_vint, HEAD_TYPE_FILE as RAR5_FILE, RAR5_SIGNATURE};

enum Command {
    List,
    Verbose,
    Test,
    ExtractFull,
    ExtractFlat,
}

struct UnrarArgs<'a> {
    command: Command,
    archive_file: Option<&'a [u8]>,
    dest_dir: Option<&'a [u8]>,
    overwrite: bool,
    assume_yes: bool,
}

impl<'a> Default for UnrarArgs<'a> {
    fn default() -> Self {
        Self {
            command: Command::ExtractFull,
            archive_file: None,
            dest_dir: None,
            overwrite: false,
            assume_yes: false,
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
            "UNRAR 5.0 freeware для Wunix OS\n\n\
             Использование: unrar <команда> [-<ключ>...] архив.rar [целевой_каталог/]\n\n\
             <Команды>:\n\
               e             извлечь файлы в текущий каталог (без подкаталогов)\n\
               l             показать список содержимого архива\n\
               t             проверить целостность файлов архива\n\
               v             подробный список файлов архива\n\
               x             извлечь файлы с полными путями\n\n\
             <Ключи>:\n\
               -o+           перезаписывать существующие файлы\n\
               -y            отвечать «Да» на все запросы\n\
               -?            показать эту справку\n",
            "UNRAR 5.0 freeware for Wunix OS\n\n\
             Usage: unrar <command> -<switch 1> -<switch N> archive [dest_path/]\n\n\
             <Commands>:\n\
               e             Extract files without archived paths\n\
               l[t[a],b[k]]  List contents of archive [technical[all], bare[kb]]\n\
               t             Test archive files\n\
               v[t[a],b[k]]  Verbosely list contents of archive\n\
               x             Extract files with full path\n\n\
             <Switches>:\n\
               -o+           Overwrite existing files\n\
               -y            Assume Yes on all queries\n\
               -?            Display this help\n",
        ).as_bytes(),
    );
}

fn parse_cli_args<'a>(argc: isize, argv: *const *const u8) -> UnrarArgs<'a> {
    let mut args = UnrarArgs::default();
    let mut i = 1;

    // Первый позиционный аргумент: команда
    if argc > 1 {
        let first = parse_cstr(unsafe { *argv.add(1) });
        if first == b"l" {
            args.command = Command::List;
            i = 2;
        } else if first == b"v" {
            args.command = Command::Verbose;
            i = 2;
        } else if first == b"t" {
            args.command = Command::Test;
            i = 2;
        } else if first == b"e" {
            args.command = Command::ExtractFlat;
            i = 2;
        } else if first == b"x" {
            args.command = Command::ExtractFull;
            i = 2;
        } else if first == b"-?" || first == b"--help" {
            print_help();
            exit(0);
        }
    }

    while i < argc {
        let arg = parse_cstr(unsafe { *argv.add(i as usize) });
        if arg == b"-o+" {
            args.overwrite = true;
        } else if arg == b"-y" {
            args.assume_yes = true;
        } else if !arg.starts_with(b"-") {
            if args.archive_file.is_none() {
                args.archive_file = Some(arg);
            } else if args.dest_dir.is_none() {
                args.dest_dir = Some(arg);
            }
        }
        i += 1;
    }

    args
}

fn process_rar4(fd: u64, file_size: u64, args: &UnrarArgs) -> u64 {
    let mut offset = RAR4_SIGNATURE.len() as u64;
    let mut files_count = 0usize;
    let mut total_unp = 0u64;
    let mut total_pack = 0u64;

    let is_list = matches!(args.command, Command::List | Command::Verbose);
    let test_only = matches!(args.command, Command::Test);
    let flat = matches!(args.command, Command::ExtractFlat);

    if is_list {
        list::print_rar_header();
    }

    while offset < file_size {
        lseek(fd, offset as i64, SEEK_SET as i32);
        let mut blk_buf = [0u8; 7];
        if read(fd, &mut blk_buf) < 7 {
            break;
        }

        let blk = match parse_block_header(&blk_buf) {
            Some(b) => b,
            None => break,
        };

        if blk.head_type == HEAD_TYPE_ENDARC {
            break;
        }

        if blk.head_type == HEAD_TYPE_FILE {
            let mut file_hdr_buf = [0u8; 512];
            let rd = read(fd, &mut file_hdr_buf);
            let slice = &file_hdr_buf[..rd as usize];

            if let Some((entry, name_off)) = parse_file_header(slice, blk.flags, offset + blk.head_size as u64) {
                let name_end = (name_off + entry.name_len).min(slice.len());
                let name_slice = &slice[name_off..name_end];

                if is_list {
                    list::print_rar_entry(
                        name_slice,
                        entry.unp_size,
                        entry.pack_size,
                        entry.file_crc,
                        entry.is_dir,
                        entry.file_time,
                    );
                } else {
                    let _ = extract::extract_rar_file(
                        fd,
                        entry.data_offset,
                        entry.pack_size,
                        entry.unp_size,
                        entry.method,
                        entry.file_crc,
                        name_slice,
                        entry.is_dir,
                        args.dest_dir,
                        test_only,
                        args.overwrite,
                        flat,
                        true,
                    );
                }

                total_unp += entry.unp_size;
                total_pack += entry.pack_size;
                files_count += 1;

                offset += blk.head_size as u64 + entry.pack_size;
                continue;
            }
        }

        offset += blk.head_size as u64;
    }

    if is_list {
        list::print_rar_footer(files_count, total_unp, total_pack);
    }

    0
}

fn process_rar5(fd: u64, file_size: u64, args: &UnrarArgs) -> u64 {
    let mut offset = RAR5_SIGNATURE.len() as u64;
    let mut files_count = 0usize;
    let mut total_unp = 0u64;
    let mut total_pack = 0u64;

    let is_list = matches!(args.command, Command::List | Command::Verbose);
    let test_only = matches!(args.command, Command::Test);
    let flat = matches!(args.command, Command::ExtractFlat);

    if is_list {
        list::print_rar_header();
    }

    while offset < file_size {
        lseek(fd, offset as i64, SEEK_SET as i32);
        let mut hdr_buf = [0u8; 1024];
        let rd = read(fd, &mut hdr_buf);
        if rd < 8 {
            break;
        }

        let mut v_off = 4usize; // Пропускаем CRC32 заголовка (4 байта)
        let head_size = match read_vint(&hdr_buf[..rd as usize], &mut v_off) {
            Some(s) => s,
            None => break,
        };
        let head_type = match read_vint(&hdr_buf[..rd as usize], &mut v_off) {
            Some(t) => t,
            None => break,
        };
        let _head_flags = match read_vint(&hdr_buf[..rd as usize], &mut v_off) {
            Some(f) => f,
            None => break,
        };

        if head_type == 5 {
            // End of archive
            break;
        }

        if head_type == RAR5_FILE {
            let data_offset = offset + 4 + head_size;
            if let Some((entry, consumed)) = rar5::parse_rar5_file_header(&hdr_buf[v_off..rd as usize], data_offset) {
                let name_start = v_off + consumed;
                let name_end = (name_start + entry.name_len).min(rd as usize);
                let name_slice = &hdr_buf[name_start..name_end];

                if is_list {
                    list::print_rar_entry(
                        name_slice,
                        entry.unp_size,
                        entry.data_size,
                        entry.file_crc,
                        entry.is_dir,
                        entry.mtime,
                    );
                } else {
                    let _ = extract::extract_rar_file(
                        fd,
                        entry.data_offset,
                        entry.data_size,
                        entry.unp_size,
                        entry.comp_info as u8,
                        entry.file_crc,
                        name_slice,
                        entry.is_dir,
                        args.dest_dir,
                        test_only,
                        args.overwrite,
                        flat,
                        true,
                    );
                }

                total_unp += entry.unp_size;
                total_pack += entry.data_size;
                files_count += 1;

                offset = data_offset + entry.data_size;
                continue;
            }
        }

        offset += 4 + head_size;
    }

    if is_list {
        list::print_rar_footer(files_count, total_unp, total_pack);
    }

    0
}

fn run_unrar(args: UnrarArgs) -> u64 {
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

    let fd_res = open(&path_nt[..len + 1], O_RDONLY as u64, 0);
    if fd_res < 0 {
        write(STDERR_FILENO, b"unrar: ");
        write(
            STDERR_FILENO,
            t("не удалось открыть файл архива\n", "cannot open archive file\n").as_bytes(),
        );
        return 1;
    }
    let fd = fd_res as u64;

    let file_size = lseek(fd, 0, SEEK_END as i32) as u64;
    lseek(fd, 0, SEEK_SET as i32);

    let mut sig = [0u8; 8];
    if read(fd, &mut sig) < 7 {
        close(fd);
        write(STDERR_FILENO, b"unrar: archive too small\n");
        return 1;
    }

    let code = if sig[0..7] == RAR4_SIGNATURE {
        process_rar4(fd, file_size, &args)
    } else if sig[0..8] == RAR5_SIGNATURE {
        process_rar5(fd, file_size, &args)
    } else {
        write(STDERR_FILENO, b"unrar: not a valid RAR archive (invalid magic signature)\n");
        1
    };

    close(fd);
    code
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc <= 1 || argv.is_null() {
        print_help();
        exit(1);
    }
    let args = parse_cli_args(argc, argv);
    let code = run_unrar(args);
    exit(code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in unrar\n");
    exit(1);
}
