// Форматирование и вывод списка содержимого архива tar (-t, -tv).

use libwunix::{fmt, write, STDOUT_FILENO};
use crate::header::{cstr_to_slice, TarEntryHeader, TYPE_DIRECTORY, TYPE_SYMLINK};

pub fn format_mode(mode: u32, typeflag: u8, out: &mut [u8; 10]) {
    out[0] = match typeflag {
        TYPE_DIRECTORY => b'd',
        TYPE_SYMLINK => b'l',
        _ => b'-',
    };
    out[1] = if (mode & 0o400) != 0 { b'r' } else { b'-' };
    out[2] = if (mode & 0o200) != 0 { b'w' } else { b'-' };
    out[3] = if (mode & 0o100) != 0 { b'x' } else { b'-' };
    out[4] = if (mode & 0o040) != 0 { b'r' } else { b'-' };
    out[5] = if (mode & 0o020) != 0 { b'w' } else { b'-' };
    out[6] = if (mode & 0o010) != 0 { b'x' } else { b'-' };
    out[7] = if (mode & 0o004) != 0 { b'r' } else { b'-' };
    out[8] = if (mode & 0o002) != 0 { b'w' } else { b'-' };
    out[9] = if (mode & 0o001) != 0 { b'x' } else { b'-' };
}

pub fn list_entry(header: &TarEntryHeader, verbose: bool) {
    let name_bytes = cstr_to_slice(&header.name);
    let prefix_bytes = cstr_to_slice(&header.prefix);

    if verbose {
        let mut mode_str = [b'-'; 10];
        format_mode(header.mode, header.typeflag, &mut mode_str);
        write(STDOUT_FILENO, &mode_str);
        write(STDOUT_FILENO, b" ");

        // Выводим uid/gid
        let mut uid_buf = [0u8; 16];
        let uid_str = fmt::utoa(header.uid as u64, &mut uid_buf);
        write(STDOUT_FILENO, uid_str.as_bytes());
        write(STDOUT_FILENO, b"/");
        let mut gid_buf = [0u8; 16];
        let gid_str = fmt::utoa(header.gid as u64, &mut gid_buf);
        write(STDOUT_FILENO, gid_str.as_bytes());
        write(STDOUT_FILENO, b" ");

        // Размер
        let mut size_buf = [0u8; 16];
        let size_str = fmt::utoa(header.size, &mut size_buf);
        for _ in size_str.len()..8 {
            write(STDOUT_FILENO, b" ");
        }
        write(STDOUT_FILENO, size_str.as_bytes());
        write(STDOUT_FILENO, b" ");
    }

    if !prefix_bytes.is_empty() {
        write(STDOUT_FILENO, prefix_bytes);
        write(STDOUT_FILENO, b"/");
    }
    write(STDOUT_FILENO, name_bytes);

    if header.typeflag == TYPE_SYMLINK {
        write(STDOUT_FILENO, b" -> ");
        write(STDOUT_FILENO, cstr_to_slice(&header.linkname));
    }

    write(STDOUT_FILENO, b"\n");
}
