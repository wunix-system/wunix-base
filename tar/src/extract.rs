// Извлечение файлов из архива tar (-x).
// Включает защиту от Directory Traversal (отсечение ../ и абсолютных путей),
// рекурсивное создание каталогов, распаковку симлинков и восстановление прав.

use libwunix::{
    chmod, close, mkdir, open, read, symlink, t, write, O_CREAT, O_TRUNC, O_WRONLY,
    STDERR_FILENO, STDOUT_FILENO,
};
use crate::header::{
    cstr_to_slice, TarEntryHeader, BLOCK_SIZE, TYPE_DIRECTORY, TYPE_REGULAR,
    TYPE_REGULAR_ALT, TYPE_SYMLINK,
};

// Проверяет путь на отсутствие попыток выхода за пределы целевой директории
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

// Создает родительские каталоги для пути
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

pub fn extract_file_data(tar_fd: u64, size: u64, dest_fd: i64) -> Result<(), ()> {
    let mut remaining = size;
    let mut block = [0u8; BLOCK_SIZE];

    while remaining > 0 {
        let to_read = BLOCK_SIZE;
        let rd = read(tar_fd, &mut block);
        if rd < to_read as u64 {
            return Err(());
        }
        let chunk = (remaining as usize).min(BLOCK_SIZE);
        if dest_fd >= 0 {
            let wr = write(dest_fd as u64, &block[..chunk]);
            if wr < chunk as u64 {
                return Err(());
            }
        }
        remaining -= chunk as u64;
    }

    Ok(())
}

pub fn extract_entry(tar_fd: u64, header: &TarEntryHeader, verbose: bool) -> Result<(), ()> {
    let name = cstr_to_slice(&header.name);
    let mut full_path = [0u8; 512];
    let mut pos = 0;

    let prefix = cstr_to_slice(&header.prefix);
    if !prefix.is_empty() {
        let p_len = prefix.len().min(250);
        full_path[pos..pos + p_len].copy_from_slice(&prefix[..p_len]);
        pos += p_len;
        full_path[pos] = b'/';
        pos += 1;
    }

    let n_len = name.len().min(250);
    full_path[pos..pos + n_len].copy_from_slice(&name[..n_len]);
    pos += n_len;

    // Отсекаем ведущие '/' и './'
    let mut start = 0;
    while start < pos && full_path[start] == b'/' {
        start += 1;
    }
    if start + 1 < pos && full_path[start] == b'.' && full_path[start + 1] == b'/' {
        start += 2;
    }

    let clean_path = &full_path[start..pos];
    if clean_path.is_empty() || !is_safe_path(clean_path) {
        // Пропускаем небезопасный путь или тело файла
        skip_file_data(tar_fd, header.size)?;
        return Ok(());
    }

    if verbose {
        write(STDOUT_FILENO, clean_path);
        write(STDOUT_FILENO, b"\n");
    }

    let mut path_nt = [0u8; 513];
    path_nt[..clean_path.len()].copy_from_slice(clean_path);
    path_nt[clean_path.len()] = 0;
    let path_slice = &path_nt[..clean_path.len() + 1];

    match header.typeflag {
        TYPE_DIRECTORY => {
            ensure_parent_dirs(clean_path);
            mkdir(path_slice, header.mode);
        }
        TYPE_SYMLINK => {
            ensure_parent_dirs(clean_path);
            let target = cstr_to_slice(&header.linkname);
            let mut target_nt = [0u8; 256];
            let t_len = target.len().min(255);
            target_nt[..t_len].copy_from_slice(&target[..t_len]);
            target_nt[t_len] = 0;
            symlink(&target_nt[..t_len + 1], path_slice);
        }
        TYPE_REGULAR | TYPE_REGULAR_ALT => {
            ensure_parent_dirs(clean_path);
            let fd = open(path_slice, (O_CREAT | O_WRONLY | O_TRUNC) as u64, header.mode as u64);
            if fd < 0 {
                write(STDERR_FILENO, b"tar: ");
                write(
                    STDERR_FILENO,
                    t("ошибка создания файла: ", "error creating file: ").as_bytes(),
                );
                write(STDERR_FILENO, clean_path);
                write(STDERR_FILENO, b"\n");
                skip_file_data(tar_fd, header.size)?;
                return Err(());
            }

            let res = extract_file_data(tar_fd, header.size, fd);
            close(fd as u64);
            chmod(path_slice, header.mode);
            return res;
        }
        _ => {
            // Другие типы файлов (блоки, устройства) пропускаем по телу
            skip_file_data(tar_fd, header.size)?;
        }
    }

    Ok(())
}

pub fn skip_file_data(tar_fd: u64, size: u64) -> Result<(), ()> {
    let blocks = (size + 511) / 512;
    let mut block = [0u8; BLOCK_SIZE];
    for _ in 0..blocks {
        let rd = read(tar_fd, &mut block);
        if rd < BLOCK_SIZE as u64 {
            return Err(());
        }
    }
    Ok(())
}
