// Вывод информации о содержимом RAR архива (команды l и v).

use libwunix::{fmt, write, STDOUT_FILENO};

pub fn print_rar_header() {
    write(STDOUT_FILENO, b" Attributes      Size     Packed Ratio   Date       Time   Checksum  Name\n");
    write(STDOUT_FILENO, b"----------- --------- ---------- ----- ---------- -------- --------  ----\n");
}

pub fn print_rar_entry(
    name: &[u8],
    unp_size: u64,
    pack_size: u64,
    crc32: u32,
    is_dir: bool,
    dos_time: u32,
) {
    let mut num_buf = [0u8; 16];

    // Атрибуты
    let attr_str = if is_dir { b"drwxr-xr-x " } else { b"-rw-r--r-- " };
    write(STDOUT_FILENO, attr_str);

    // Размер исходный
    let u_str = fmt::utoa(unp_size, &mut num_buf);
    for _ in u_str.len()..9 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, u_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    // Размер сжатый
    let p_str = fmt::utoa(pack_size, &mut num_buf);
    for _ in p_str.len()..10 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, p_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    // Процент сжатия
    let ratio = if unp_size > 0 {
        100 - (pack_size * 100 / unp_size)
    } else {
        0
    };
    let r_str = fmt::utoa(ratio, &mut num_buf);
    for _ in r_str.len()..3 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, r_str.as_bytes());
    write(STDOUT_FILENO, b"% ");

    // Дата и время (из DOS формата)
    let date = (dos_time >> 16) as u16;
    let time = (dos_time & 0xFFFF) as u16;
    let year = 1980 + ((date >> 9) & 0x7F);
    let month = (date >> 5) & 0x0F;
    let day = date & 0x1F;
    let hour = (time >> 11) & 0x1F;
    let min = (time >> 5) & 0x3F;

    let y_str = fmt::utoa(year as u64, &mut num_buf);
    write(STDOUT_FILENO, y_str.as_bytes());
    write(STDOUT_FILENO, b"-");

    let m_str = fmt::utoa(month as u64, &mut num_buf);
    if m_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, m_str.as_bytes());
    write(STDOUT_FILENO, b"-");

    let d_str = fmt::utoa(day as u64, &mut num_buf);
    if d_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, d_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    let h_str = fmt::utoa(hour as u64, &mut num_buf);
    if h_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, h_str.as_bytes());
    write(STDOUT_FILENO, b":");

    let min_str = fmt::utoa(min as u64, &mut num_buf);
    if min_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, min_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    // CRC32 в hex
    let crc_str = fmt::utoa_hex(crc32 as u64, &mut num_buf);
    for _ in crc_str.len()..8 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, crc_str.as_bytes());
    write(STDOUT_FILENO, b"  ");

    // Имя файла
    write(STDOUT_FILENO, name);
    write(STDOUT_FILENO, b"\n");
}

pub fn print_rar_footer(files: usize, total_unp: u64, total_pack: u64) {
    let mut num_buf = [0u8; 16];
    write(STDOUT_FILENO, b"----------- --------- ---------- ----- ---------- -------- --------  ----\n");

    write(STDOUT_FILENO, b"            ");
    let u_str = fmt::utoa(total_unp, &mut num_buf);
    for _ in u_str.len()..9 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, u_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    let p_str = fmt::utoa(total_pack, &mut num_buf);
    for _ in p_str.len()..10 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, p_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    let ratio = if total_unp > 0 {
        100 - (total_pack * 100 / total_unp)
    } else {
        0
    };
    let r_str = fmt::utoa(ratio, &mut num_buf);
    for _ in r_str.len()..3 {
        write(STDOUT_FILENO, b" ");
    }
    write(STDOUT_FILENO, r_str.as_bytes());
    write(STDOUT_FILENO, b"%                                    ");

    let f_str = fmt::utoa(files as u64, &mut num_buf);
    write(STDOUT_FILENO, f_str.as_bytes());
    write(STDOUT_FILENO, b" files\n");
}
