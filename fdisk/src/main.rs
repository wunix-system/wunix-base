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

fn type_name(id: u8) -> &'static str {
    match id {
        0x01 => "FAT12",
        0x04 => "FAT16 <32M",
        0x06 => "FAT16",
        0x07 => "HPFS/NTFS/exFAT",
        0x0B => "W95 FAT32",
        0x0C => "W95 FAT32 (LBA)",
        0x0E => "W95 FAT16 (LBA)",
        0x82 => "Linux swap",
        0x83 => "Linux",
        0xEE => "GPT",
        _ => "Unknown",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut dev_path: &'static [u8] = b"/dev/vda";
    let mut list_mode = false;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: fdisk [ОПЦИИ] <устройство>\n\
                     Опции:\n  \
                       -l, --list      Вывести таблицу разделов\n  \
                       -h, --help      Показать эту справку\n  \
                       -V, --version   Версия fdisk",
                    "Usage: fdisk [OPTIONS] <device>\n\
                     Options:\n  \
                       -l, --list      List partition table(s)\n  \
                       -h, --help      Display this help and exit\n  \
                       -V, --version   Display version"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("fdisk (Wunix util-linux) 2.39.0");
            exit(0);
        } else if arg == b"-l" || arg == b"--list" {
            list_mode = true;
        } else if !arg.starts_with(b"-") {
            dev_path = arg;
        }
        i += 1;
    }

    let mut null_term = [0u8; 256];
    let copy_len = dev_path.len().min(255);
    null_term[..copy_len].copy_from_slice(&dev_path[..copy_len]);
    null_term[copy_len] = 0;

    let fd = open(&null_term[..copy_len + 1], 0 /* O_RDONLY */, 0);
    if fd < 0 {
        println!(
            "{}: {}",
            t("fdisk: не удалось открыть устройство", "fdisk: cannot open device"),
            core::str::from_utf8(dev_path).unwrap_or("")
        );
        exit(1);
    }

    let total_bytes = libwunix::lseek(fd as u64, 0, libwunix::SEEK_END);
    let _ = libwunix::lseek(fd as u64, 0, libwunix::SEEK_SET);

    let mut sector0 = [0u8; 512];
    let n = read(fd as u64, &mut sector0);
    close(fd as u64);

    if n < 512 {
        println!(
            "{}",
            t("fdisk: ошибка чтения сектора MBR", "fdisk: read error")
        );
        exit(1);
    }

    let dev_s = core::str::from_utf8(dev_path).unwrap_or("/dev/vda");

    if sector0[510] != 0x55 || sector0[511] != 0xAA {
        println!(
            "{}: {} ({})",
            t("fdisk: устройство не содержит корректной сигнатуры MBR", "fdisk: invalid MBR partition table"),
            dev_s,
            "0x55AA missing"
        );
        exit(1);
    }

    let disk_id = u32::from_le_bytes([sector0[440], sector0[441], sector0[442], sector0[443]]);
    let (disk_bytes, total_sectors, total_mb) = if total_bytes > 0 {
        let b = total_bytes as u64;
        (b, b / 512, b / (1024 * 1024))
    } else {
        (0, 0, 0)
    };

    if libwunix::current_language() == libwunix::Language::Russian {
        println!("Диск {}: {} МиБ, {} байт, {} секторов", dev_s, total_mb, disk_bytes, total_sectors);
        println!("Единицы: секторов по 1 * 512 = 512 байт");
        println!("Размер сектора (логический/физический): 512 байт / 512 байт");
        println!("Размер I/O (минимальный/оптимальный): 512 байт / 512 байт");
        println!("Тип метки диска: dos");
        println!("Идентификатор диска: {:#010x}\n", disk_id);
        println!("{:<12} {:<4} {:>8} {:>8} {:>8} {:>5} {:>2} {:<20}", "Устройство", "Загр", "Начало", "Конец", "Секторы", "Размер", "Ид", "Тип");
    } else {
        println!("Disk {}: {} MiB, {} bytes, {} sectors", dev_s, total_mb, disk_bytes, total_sectors);
        println!("Units: sectors of 1 * 512 = 512 bytes");
        println!("Sector size (logical/physical): 512 bytes / 512 bytes");
        println!("I/O size (minimum/optimal): 512 bytes / 512 bytes");
        println!("Disklabel type: dos");
        println!("Disk identifier: {:#010x}\n", disk_id);
        println!("{:<12} {:<4} {:>8} {:>8} {:>8} {:>5} {:>2} {:<20}", "Device", "Boot", "Start", "End", "Sectors", "Size", "Id", "Type");
    }

    let mut found_parts = 0;
    for part_idx in 0..4 {
        let offset = 446 + part_idx * 16;
        let entry = &sector0[offset..offset + 16];

        let bootable = entry[0] == 0x80;
        let part_type = entry[4];
        let start_lba = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let sector_count = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);

        if part_type == 0 || sector_count == 0 {
            continue;
        }

        found_parts += 1;
        let end_lba = start_lba + sector_count - 1;
        let size_mb = (sector_count as u64 * 512) / (1024 * 1024);

        print!("{}{:<2} ", dev_s, part_idx + 1);
        if bootable {
            print!("*    ");
        } else {
            print!("     ");
        }
        print!("{:>8} {:>8} {:>8} {:>4}M {:>2x} {:<20}\n", start_lba, end_lba, sector_count, size_mb, part_type, type_name(part_type));
    }

    if found_parts == 0 && list_mode {
        println!("{}", t("Разделы не обнаружены", "No partitions found"));
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("fdisk: panic occurred");
    exit(1);
}
