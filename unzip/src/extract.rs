// Распаковка и проверка файлов из ZIP архива.
// Поддерживает методы Store (0) и Deflate (8), проверку CRC32,
// защиту от Directory Traversal и создание вложенных директорий.

use libwunix::{
    chmod, close, lseek, mkdir, open, read, write, Crc32, O_CREAT, O_TRUNC, O_WRONLY,
    SEEK_SET, STDERR_FILENO, STDOUT_FILENO,
};
use crate::format::{
    CentralDirEntry, LOCAL_FILE_HEADER_SIG, LOCAL_HEADER_SIZE, METHOD_DEFLATE, METHOD_STORE,
};

pub fn is_safe_path(path: &[u8]) -> bool {
    let mut i = 0;
    while i < path.len() {
        if path[i] == b'.' && i + 1 < path.len() && path[i + 1] == b'.' {
            let prev_sep = i == 0 || path[i - 1] == b'/';
            let next_sep = i + 2 == path.len() || path[i + 2] == b'/';
            if prev_sep && next_sep {
                return false;
            }
        }
        i += 1;
    }
    true
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

pub fn extract_or_test_entry(
    zip_fd: u64,
    entry: &CentralDirEntry,
    name: &[u8],
    dest_dir: Option<&[u8]>,
    test_only: bool,
    overwrite: bool,
    verbose: bool,
) -> Result<(), ()> {
    if !is_safe_path(name) {
        return Err(());
    }

    // Позиционируемся на локальный заголовок файла
    lseek(zip_fd, entry.local_header_offset as i64, SEEK_SET as i32);

    let mut local_hdr = [0u8; LOCAL_HEADER_SIZE];
    if read(zip_fd, &mut local_hdr) < LOCAL_HEADER_SIZE as u64 {
        return Err(());
    }

    let sig = u32::from_le_bytes([local_hdr[0], local_hdr[1], local_hdr[2], local_hdr[3]]);
    if sig != LOCAL_FILE_HEADER_SIG {
        return Err(());
    }

    let local_name_len = u16::from_le_bytes([local_hdr[26], local_hdr[27]]) as i64;
    let local_extra_len = u16::from_le_bytes([local_hdr[28], local_hdr[29]]) as i64;

    // Пропускаем имя файла и extra поле локального заголовка
    lseek(zip_fd, local_name_len + local_extra_len, libwunix::SEEK_CUR as i32);

    // Формируем полный целевой путь
    let mut full_path = [0u8; 512];
    let mut pos = 0usize;

    if let Some(d) = dest_dir {
        let d_len = d.len().min(250);
        full_path[pos..pos + d_len].copy_from_slice(&d[..d_len]);
        pos += d_len;
        if pos > 0 && full_path[pos - 1] != b'/' {
            full_path[pos] = b'/';
            pos += 1;
        }
    }

    let n_len = name.len().min(512 - pos);
    full_path[pos..pos + n_len].copy_from_slice(&name[..n_len]);
    pos += n_len;

    let target_path = &full_path[..pos];
    let mut path_nt = [0u8; 513];
    path_nt[..pos].copy_from_slice(target_path);
    path_nt[pos] = 0;
    let path_slice = &path_nt[..pos + 1];

    if entry.is_dir(name) {
        if !test_only {
            ensure_parent_dirs(target_path);
            mkdir(path_slice, entry.mode());
            if verbose {
                write(STDOUT_FILENO, b"   creating: ");
                write(STDOUT_FILENO, target_path);
                write(STDOUT_FILENO, b"\n");
            }
        }
        return Ok(());
    }

    if test_only {
        if verbose {
            write(STDOUT_FILENO, b"    testing: ");
            write(STDOUT_FILENO, name);
            write(STDOUT_FILENO, b" ");
        }
    } else {
        if verbose {
            write(STDOUT_FILENO, b"  inflating: ");
            write(STDOUT_FILENO, target_path);
            write(STDOUT_FILENO, b"\n");
        }
    }

    // Открываем выходной файл при распаковке
    let out_fd = if !test_only {
        ensure_parent_dirs(target_path);
        let flags = (O_CREAT | O_WRONLY | if overwrite { O_TRUNC } else { 0 }) as u64;
        let fd = open(path_slice, flags, entry.mode() as u64);
        if fd < 0 {
            write(STDERR_FILENO, b"unzip: error opening ");
            write(STDERR_FILENO, target_path);
            write(STDERR_FILENO, b"\n");
            return Err(());
        }
        Some(fd as u64)
    } else {
        None
    };

    let mut hasher = Crc32::new();
    let mut decompressed_bytes = 0usize;

    match entry.method {
        METHOD_STORE => {
            let mut buf = [0u8; 4096];
            let mut remaining = entry.uncompressed_size as usize;
            while remaining > 0 {
                let to_read = remaining.min(buf.len());
                let rd = read(zip_fd, &mut buf[..to_read]);
                if rd == 0 {
                    break;
                }
                let chunk = rd as usize;
                hasher.update(&buf[..chunk]);
                decompressed_bytes += chunk;
                if let Some(fd) = out_fd {
                    write(fd, &buf[..chunk]);
                }
                remaining -= chunk;
            }
        }
        METHOD_DEFLATE => {
            // Читаем сжатые данные в буфер и распаковываем через inflate
            let comp_size = entry.compressed_size as usize;
            let mut comp_buf = [0u8; 65536];
            let to_read = comp_size.min(comp_buf.len());
            let rd = read(zip_fd, &mut comp_buf[..to_read]);
            if (rd as usize) < to_read {
                if let Some(fd) = out_fd {
                    close(fd);
                }
                return Err(());
            }

            let res = libwunix::inflate(&comp_buf[..to_read], |chunk| {
                hasher.update(chunk);
                decompressed_bytes += chunk.len();
                if let Some(fd) = out_fd {
                    write(fd, chunk);
                }
                Ok(())
            });

            if res.is_err() {
                if let Some(fd) = out_fd {
                    close(fd);
                }
                if test_only {
                    write(STDOUT_FILENO, b" failed (decompression error)\n");
                }
                return Err(());
            }
        }
        _ => {
            if let Some(fd) = out_fd {
                close(fd);
            }
            write(STDERR_FILENO, b"unzip: unsupported compression method\n");
            return Err(());
        }
    }

    if let Some(fd) = out_fd {
        close(fd);
        chmod(path_slice, entry.mode());
    }

    let calculated_crc = hasher.finish();
    if calculated_crc != entry.crc32 || decompressed_bytes != entry.uncompressed_size as usize {
        if test_only {
            write(STDOUT_FILENO, b" failed (CRC or size mismatch)\n");
        } else {
            write(STDERR_FILENO, b"unzip: CRC32 or size mismatch for ");
            write(STDERR_FILENO, name);
            write(STDERR_FILENO, b"\n");
        }
        return Err(());
    }

    if test_only && verbose {
        write(STDOUT_FILENO, b" OK\n");
    }

    Ok(())
}
