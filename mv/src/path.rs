pub const MAX_PATH: usize = 1024;

pub const S_IFMT: u32 = 0o170000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFDIR: u32 = 0o040000;

pub fn is_dir_mode(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFDIR
}

pub fn is_lnk_mode(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFLNK
}

pub fn is_reg_mode(mode: u32) -> bool {
    (mode & S_IFMT) == S_IFREG
}

pub fn to_cstr<'a>(src: &[u8], buf: &'a mut [u8; MAX_PATH]) -> Option<&'a [u8]> {
    if src.is_empty() || src.len() >= buf.len() {
        return None;
    }
    buf[..src.len()].copy_from_slice(src);
    buf[src.len()] = 0;
    Some(&buf[..src.len() + 1])
}

/// Возвращает последний компонент пути без завершающих '/'.
pub fn basename(path: &[u8]) -> &[u8] {
    let mut end = path.len();
    while end > 1 && path[end - 1] == b'/' {
        end -= 1;
    }
    let trimmed = &path[..end];
    if trimmed == b"/" {
        return b"/";
    }
    match trimmed.iter().rposition(|&b| b == b'/') {
        Some(idx) => &trimmed[idx + 1..],
        None => trimmed,
    }
}

/// Соединяет каталог и имя файла в буфер (без завершающего '\0').
pub fn join_path<'a>(dir: &[u8], name: &[u8], out: &'a mut [u8; MAX_PATH]) -> Option<&'a [u8]> {
    let mut len = 0usize;
    if dir.len() >= out.len() {
        return None;
    }
    out[..dir.len()].copy_from_slice(dir);
    len += dir.len();

    if !dir.ends_with(b"/") {
        if len + 1 >= out.len() {
            return None;
        }
        out[len] = b'/';
        len += 1;
    }

    if len + name.len() >= out.len() {
        return None;
    }
    out[len..len + name.len()].copy_from_slice(name);
    len += name.len();
    Some(&out[..len])
}
