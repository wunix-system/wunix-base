// Извлечение и тестирование файлов из архивов RAR (RAR4 и RAR5).

use libwunix::{
    chmod, close, lseek, mkdir, open, read, write, Crc32, O_CREAT, O_TRUNC, O_WRONLY,
    SEEK_SET, STDOUT_FILENO,
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

pub fn extract_rar_file(
    rar_fd: u64,
    data_offset: u64,
    pack_size: u64,
    _unp_size: u64,
    method: u8,
    file_crc: u32,
    name: &[u8],
    is_dir: bool,
    dest_dir: Option<&[u8]>,
    test_only: bool,
    overwrite: bool,
    flat: bool,
    verbose: bool,
) -> Result<(), ()> {
    if !is_safe_path(name) {
        return Err(());
    }

    // Извлекаем имя с сохранением пути или только имя файла (для команды 'e')
    let base_name = if flat {
        let last_slash = name.iter().rposition(|&b| b == b'/');
        match last_slash {
            Some(idx) => &name[idx + 1..],
            None => name,
        }
    } else {
        name
    };

    if base_name.is_empty() {
        return Ok(());
    }

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

    let n_len = base_name.len().min(512 - pos);
    full_path[pos..pos + n_len].copy_from_slice(&base_name[..n_len]);
    pos += n_len;

    let target_path = &full_path[..pos];
    let mut path_nt = [0u8; 513];
    path_nt[..pos].copy_from_slice(target_path);
    path_nt[pos] = 0;
    let path_slice = &path_nt[..pos + 1];

    if is_dir {
        if !test_only {
            ensure_parent_dirs(target_path);
            mkdir(path_slice, 0o755);
            if verbose {
                write(STDOUT_FILENO, b"Creating    ");
                write(STDOUT_FILENO, target_path);
                write(STDOUT_FILENO, b"        OK\n");
            }
        }
        return Ok(());
    }

    if verbose {
        if test_only {
            write(STDOUT_FILENO, b"Testing     ");
        } else {
            write(STDOUT_FILENO, b"Extracting  ");
        }
        write(STDOUT_FILENO, target_path);
        for _ in target_path.len()..32 {
            write(STDOUT_FILENO, b" ");
        }
    }

    lseek(rar_fd, data_offset as i64, SEEK_SET as i32);

    let out_fd = if !test_only {
        ensure_parent_dirs(target_path);
        let flags = (O_CREAT | O_WRONLY | if overwrite { O_TRUNC } else { 0 }) as u64;
        let fd = open(path_slice, flags, 0o644);
        if fd < 0 {
            if verbose {
                write(STDOUT_FILENO, b" Cannot open file\n");
            }
            return Err(());
        }
        Some(fd as u64)
    } else {
        None
    };

    let mut hasher = Crc32::new();
    let mut total_read = 0u64;

    // Метод 0x30 (RAR4 Store) или Метод 0 (RAR5 Store)
    if method == 0x30 || method == 0 {
        let mut buf = [0u8; 4096];
        while total_read < pack_size {
            let to_read = (pack_size - total_read).min(buf.len() as u64) as usize;
            let rd = read(rar_fd, &mut buf[..to_read]);
            if rd == 0 {
                break;
            }
            let chunk = rd as usize;
            hasher.update(&buf[..chunk]);
            if let Some(fd) = out_fd {
                write(fd, &buf[..chunk]);
            }
            total_read += rd;
        }

        if let Some(fd) = out_fd {
            close(fd);
            chmod(path_slice, 0o644);
        }

        let calc_crc = hasher.finish();
        if file_crc != 0 && calc_crc != file_crc {
            if verbose {
                write(STDOUT_FILENO, b" CRC Failed\n");
            }
            return Err(());
        }

        if verbose {
            write(STDOUT_FILENO, b" OK\n");
        }
        Ok(())
    } else {
        if let Some(fd) = out_fd {
            close(fd);
        }
        if verbose {
            write(STDOUT_FILENO, b" Unsupported compression method\n");
        }
        Err(())
    }
}
