#![no_std]
#![no_main]

use libwunix::{close, exit, open, print, println, read, t};

fn cstr_to_slice(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

fn parse_u64(s: &str) -> Option<u64> {
    let mut val = 0u64;
    if s.is_empty() {
        return None;
    }
    for b in s.bytes() {
        if b >= b'0' && b <= b'9' {
            val = val * 10 + (b - b'0') as u64;
        } else {
            return None;
        }
    }
    Some(val)
}

fn format_size(blocks_kb: u64, bytes_mode: bool) {
    if bytes_mode {
        print!("{:>8} ", blocks_kb * 1024);
    } else {
        let mb = blocks_kb / 1024;
        if mb >= 1024 {
            print!("{:>4}G ", mb / 1024);
        } else if mb > 0 {
            print!("{:>4}M ", mb);
        } else {
            print!("{:>4}K ", blocks_kb);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut bytes_mode = false;
    let mut list_mode = false;

    for i in 1..argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: lsblk [ОПЦИИ]\n\
                     Опции:\n  \
                       -a, --all      Показывать все устройства\n  \
                       -b, --bytes    Выводить размер в байтах\n  \
                       -l, --list     Вывод в виде плоского списка (без дерева)\n  \
                       -h, --help     Показать эту справку\n  \
                       -V, --version  Версия утилиты",
                    "Usage: lsblk [OPTIONS]\n\
                     Options:\n  \
                       -a, --all      Print all devices\n  \
                       -b, --bytes    Print SIZE in bytes\n  \
                       -l, --list     Use list format output\n  \
                       -h, --help     Display this help and exit\n  \
                       -V, --version  Display version"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("lsblk (Wunix util-linux) 1.0.0");
            exit(0);
        } else if arg == b"-b" || arg == b"--bytes" {
            bytes_mode = true;
        } else if arg == b"-l" || arg == b"--list" {
            list_mode = true;
        }
    }

    println!(
        "{}",
        t(
            "ИМЯ        MAJ:MIN RM РАЗМЕР RO ТИП  ТОЧКА_МОНТИРОВАНИЯ",
            "NAME       MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS"
        )
    );

    // Читаем список точек монтирования из /proc/mounts
    let mut mounts_buf = [0u8; 2048];
    let mut mounts_len = 0;
    let m_fd = open(b"/proc/mounts\0", 0, 0);
    if m_fd >= 0 {
        let n = read(m_fd as u64, &mut mounts_buf);
        if n > 0 {
            mounts_len = n as usize;
        }
        close(m_fd as u64);
    }
    let mounts_str = core::str::from_utf8(&mounts_buf[..mounts_len]).unwrap_or("");

    // Читаем блочные устройства из /proc/partitions
    let mut part_buf = [0u8; 4096];
    let mut part_len = 0;
    let p_fd = open(b"/proc/partitions\0", 0, 0);
    if p_fd >= 0 {
        let n = read(p_fd as u64, &mut part_buf);
        if n > 0 {
            part_len = n as usize;
        }
        close(p_fd as u64);
    }

    if part_len == 0 {
        exit(0);
    }

    let part_str = core::str::from_utf8(&part_buf[..part_len]).unwrap_or("");

    for (idx, line) in part_str.lines().enumerate() {
        if idx == 0 {
            continue; // Заголовок
        }
        let mut fields = line.split_whitespace();
        let maj = match fields.next() {
            Some(m) => m,
            None => continue,
        };
        let min = match fields.next() {
            Some(m) => m,
            None => continue,
        };
        let blocks_str = match fields.next() {
            Some(b) => b,
            None => continue,
        };
        let name = match fields.next() {
            Some(n) => n,
            None => continue,
        };

        let blocks_kb = parse_u64(blocks_str).unwrap_or(0);
        let is_part = name.bytes().any(|b| b >= b'0' && b <= b'9');

        // Ищем точку монтирования в mounts_str
        let mut mount_point = "";
        for m_line in mounts_str.lines() {
            let mut m_fields = m_line.split_whitespace();
            if let Some(src) = m_fields.next() {
                if src.ends_with(name) {
                    if let Some(tgt) = m_fields.next() {
                        mount_point = tgt;
                        break;
                    }
                }
            }
        }

        if is_part && !list_mode {
            print!("└─{:<8} ", name);
        } else {
            print!("{:<10} ", name);
        }

        // MAJ:MIN
        let mut maj_min_buf = [0u8; 16];
        let mut m_len = 0;
        for b in maj.bytes() { maj_min_buf[m_len] = b; m_len += 1; }
        maj_min_buf[m_len] = b':'; m_len += 1;
        for b in min.bytes() { maj_min_buf[m_len] = b; m_len += 1; }
        let maj_min = core::str::from_utf8(&maj_min_buf[..m_len]).unwrap_or("");

        print!("{:>7}  0 ", maj_min);
        format_size(blocks_kb, bytes_mode);
        print!(" 0 {:<4} {:<16}\n", if is_part { "part" } else { "disk" }, mount_point);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("lsblk: panic occurred");
    exit(1);
}
