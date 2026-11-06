// Вывод содержимого ZIP архива (-l, -v).

use libwunix::{fmt, write, STDOUT_FILENO};
use crate::format::CentralDirEntry;

pub fn print_list_header(verbose: bool) {
    if verbose {
        write(STDOUT_FILENO, b" Length   Method    Size  Ratio   Date   Time   CRC-32    Name\n");
        write(STDOUT_FILENO, b"--------  ------  ------- ----- ---------- ----- --------  ----\n");
    } else {
        write(STDOUT_FILENO, b"  Length      Date    Time    Name\n");
        write(STDOUT_FILENO, b"---------  ---------- -----   ----\n");
    }
}

pub fn print_entry_line(entry: &CentralDirEntry, name: &[u8], verbose: bool) {
    let mut num_buf = [0u8; 16];

    if verbose {
        let u_str = fmt::utoa(entry.uncompressed_size as u64, &mut num_buf);
        for _ in u_str.len()..8 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, u_str.as_bytes());
        write(STDOUT_FILENO, b"  ");

        let method_str = if entry.method == 8 {
            b"Defl:N"
        } else {
            b"Stored"
        };
        write(STDOUT_FILENO, method_str);
        write(STDOUT_FILENO, b"  ");

        let c_str = fmt::utoa(entry.compressed_size as u64, &mut num_buf);
        for _ in c_str.len()..7 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, c_str.as_bytes());
        write(STDOUT_FILENO, b" ");

        // Процент сжатия
        let ratio = if entry.uncompressed_size > 0 {
            100 - (entry.compressed_size * 100 / entry.uncompressed_size)
        } else {
            0
        };
        let r_str = fmt::utoa(ratio as u64, &mut num_buf);
        for _ in r_str.len()..3 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, r_str.as_bytes());
        write(STDOUT_FILENO, b"% ");

        // Дата и время DOS:
        // mod_date: bits 0-4 day, 5-8 month, 9-15 year-1980
        // mod_time: bits 0-4 sec/2, 5-10 min, 11-15 hour
        print_dos_datetime(entry.mod_date, entry.mod_time);
        write(STDOUT_FILENO, b" ");

        let crc_str = fmt::utoa_hex(entry.crc32 as u64, &mut num_buf);
        for _ in crc_str.len()..8 {
            write(STDOUT_FILENO, b"0");
        }
        write(STDOUT_FILENO, crc_str.as_bytes());
        write(STDOUT_FILENO, b"  ");
    } else {
        let u_str = fmt::utoa(entry.uncompressed_size as u64, &mut num_buf);
        for _ in u_str.len()..9 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, u_str.as_bytes());
        write(STDOUT_FILENO, b"  ");

        print_dos_datetime(entry.mod_date, entry.mod_time);
        write(STDOUT_FILENO, b"   ");
    }

    write(STDOUT_FILENO, name);
    write(STDOUT_FILENO, b"\n");
}

fn print_dos_datetime(date: u16, time: u16) {
    let year = 1980 + ((date >> 9) & 0x7F);
    let month = (date >> 5) & 0x0F;
    let day = date & 0x1F;

    let hour = (time >> 11) & 0x1F;
    let min = (time >> 5) & 0x3F;

    let mut buf = [0u8; 16];
    let y_str = fmt::utoa(year as u64, &mut buf);
    write(STDOUT_FILENO, y_str.as_bytes());
    write(STDOUT_FILENO, b"-");

    let m_str = fmt::utoa(month as u64, &mut buf);
    if m_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, m_str.as_bytes());
    write(STDOUT_FILENO, b"-");

    let d_str = fmt::utoa(day as u64, &mut buf);
    if d_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, d_str.as_bytes());
    write(STDOUT_FILENO, b" ");

    let h_str = fmt::utoa(hour as u64, &mut buf);
    if h_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, h_str.as_bytes());
    write(STDOUT_FILENO, b":");

    let min_str = fmt::utoa(min as u64, &mut buf);
    if min_str.len() == 1 {
        write(STDOUT_FILENO, b"0");
    }
    write(STDOUT_FILENO, min_str.as_bytes());
}

pub fn print_list_footer(total_files: usize, total_uncompressed: u64, total_compressed: u64, verbose: bool) {
    let mut num_buf = [0u8; 16];
    if verbose {
        write(STDOUT_FILENO, b"--------          ------- -----                          ----\n");
        let u_str = fmt::utoa(total_uncompressed, &mut num_buf);
        for _ in u_str.len()..8 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, u_str.as_bytes());
        write(STDOUT_FILENO, b"                  ");

        let c_str = fmt::utoa(total_compressed, &mut num_buf);
        for _ in c_str.len()..7 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, c_str.as_bytes());
        write(STDOUT_FILENO, b" ");

        let ratio = if total_uncompressed > 0 {
            100 - (total_compressed * 100 / total_uncompressed)
        } else {
            0
        };
        let r_str = fmt::utoa(ratio, &mut num_buf);
        for _ in r_str.len()..3 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, r_str.as_bytes());
        write(STDOUT_FILENO, b"%                          ");

        let f_str = fmt::utoa(total_files as u64, &mut num_buf);
        write(STDOUT_FILENO, f_str.as_bytes());
        write(STDOUT_FILENO, b" files\n");
    } else {
        write(STDOUT_FILENO, b"---------                     -------\n");
        let u_str = fmt::utoa(total_uncompressed, &mut num_buf);
        for _ in u_str.len()..9 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, u_str.as_bytes());
        write(STDOUT_FILENO, b"                     ");
        let f_str = fmt::utoa(total_files as u64, &mut num_buf);
        write(STDOUT_FILENO, f_str.as_bytes());
        write(STDOUT_FILENO, b" files\n");
    }
}
