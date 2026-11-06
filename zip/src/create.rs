// Создание архива ZIP.
// Поддерживает сжатие DEFLATE (метод 8) и Store (метод 0),
// запись заголовков локальных файлов, центрального каталога и EOCD.

use libwunix::{
    close, getdents64, lstat, open, read, write, Crc32, Stat, O_RDONLY, STDOUT_FILENO,
};
use crate::format::{
    write_central_header, write_eocd, write_local_header, ZipEntryMeta, CENTRAL_HEADER_SIZE,
    EOCD_SIZE, LOCAL_HEADER_SIZE, METHOD_DEFLATE, METHOD_STORE,
};

pub const MAX_ENTRIES: usize = 256;
pub const MAX_NAME_STORAGE: usize = 16384;

pub struct ZipArchiveState {
    pub entries: [ZipEntryMeta; MAX_ENTRIES],
    pub entry_count: usize,
    pub name_storage: [u8; MAX_NAME_STORAGE],
    pub name_storage_len: usize,
    pub current_offset: u32,
}

impl ZipArchiveState {
    pub const fn new() -> Self {
        Self {
            entries: [ZipEntryMeta {
                name_offset: 0,
                name_len: 0,
                method: METHOD_STORE,
                crc32: 0,
                compressed_size: 0,
                uncompressed_size: 0,
                local_header_offset: 0,
                mode: 0o644,
                is_dir: false,
            }; MAX_ENTRIES],
            entry_count: 0,
            name_storage: [0u8; MAX_NAME_STORAGE],
            name_storage_len: 0,
            current_offset: 0,
        }
    }

    pub fn record_name(&mut self, name: &[u8]) -> Option<(usize, usize)> {
        if self.name_storage_len + name.len() > MAX_NAME_STORAGE {
            return None;
        }
        let start = self.name_storage_len;
        self.name_storage[start..start + name.len()].copy_from_slice(name);
        self.name_storage_len += name.len();
        Some((start, name.len()))
    }
}

pub fn add_zip_file(
    out_fd: u64,
    state: &mut ZipArchiveState,
    path: &[u8],
    store_only: bool,
    verbose: bool,
) -> Result<(), ()> {
    if state.entry_count >= MAX_ENTRIES {
        return Err(());
    }

    let mut path_nt = [0u8; 513];
    let p_len = path.len().min(512);
    path_nt[..p_len].copy_from_slice(&path[..p_len]);
    path_nt[p_len] = 0;
    let path_slice = &path_nt[..p_len + 1];

    let mut st = Stat {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 0,
        st_mode: 0,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: 0,
        st_blksize: 0,
        st_blocks: 0,
        st_atime: 0,
        st_atime_nsec: 0,
        st_mtime: 0,
        st_mtime_nsec: 0,
        st_ctime: 0,
        st_ctime_nsec: 0,
        __unused: [0; 3],
    };

    if lstat(path_slice, &mut st) != 0 {
        return Err(());
    }

    let is_dir = (st.st_mode & 0o170000) == 0o040000;
    let mode = (st.st_mode & 0o7777) as u32;

    // Нормализуем имя для ZIP (директории заканчиваются на '/')
    let mut zip_name = [0u8; 512];
    let mut z_len = path.len().min(510);
    zip_name[..z_len].copy_from_slice(&path[..z_len]);
    if is_dir && zip_name[z_len - 1] != b'/' {
        zip_name[z_len] = b'/';
        z_len += 1;
    }

    let (name_offset, name_len) = state.record_name(&zip_name[..z_len]).ok_or(())?;
    let local_header_offset = state.current_offset;

    if is_dir {
        // Каталог в ZIP сохраняется с пустым телом
        let mut local_hdr = [0u8; LOCAL_HEADER_SIZE];
        write_local_header(&mut local_hdr, METHOD_STORE, 0, 0, 0, name_len as u16);
        write(out_fd, &local_hdr);
        write(out_fd, &zip_name[..z_len]);
        state.current_offset += (LOCAL_HEADER_SIZE + name_len) as u32;

        state.entries[state.entry_count] = ZipEntryMeta {
            name_offset,
            name_len,
            method: METHOD_STORE,
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
            local_header_offset,
            mode,
            is_dir: true,
        };
        state.entry_count += 1;

        if verbose {
            write(STDOUT_FILENO, b"  adding: ");
            write(STDOUT_FILENO, &zip_name[..z_len]);
            write(STDOUT_FILENO, b" (stored 0%)\n");
        }

        // Рекурсивно сканируем подкаталог
        let dir_fd = open(path_slice, O_RDONLY as u64, 0);
        if dir_fd >= 0 {
            let mut buf = [0u8; 2048];
            loop {
                let nread = getdents64(dir_fd as u64, &mut buf);
                if nread <= 0 {
                    break;
                }
                let mut offset = 0usize;
                while offset < nread as usize {
                    let d_reclen = (buf[offset + 16] as usize) | ((buf[offset + 17] as usize) << 8);
                    if d_reclen == 0 {
                        break;
                    }
                    let d_name = &buf[offset + 19..offset + d_reclen];
                    let name_end = d_name.iter().position(|&b| b == 0).unwrap_or(d_name.len());
                    let child_name = &d_name[..name_end];

                    if child_name != b"." && child_name != b".." {
                        let mut subpath = [0u8; 512];
                        let mut sub_len = path.len();
                        subpath[..sub_len].copy_from_slice(path);
                        if subpath[sub_len - 1] != b'/' {
                            subpath[sub_len] = b'/';
                            sub_len += 1;
                        }
                        let c_len = child_name.len().min(512 - sub_len);
                        subpath[sub_len..sub_len + c_len].copy_from_slice(&child_name[..c_len]);
                        sub_len += c_len;

                        let _ = add_zip_file(out_fd, state, &subpath[..sub_len], store_only, verbose);
                    }

                    offset += d_reclen;
                }
            }
            close(dir_fd as u64);
        }

        return Ok(());
    }

    // Обычный файл: читаем содержимое и вычисляем CRC32
    let fd = open(path_slice, O_RDONLY as u64, 0);
    if fd < 0 {
        return Err(());
    }

    let uncompressed_size = st.st_size as u32;
    let mut file_data = [0u8; 65536];
    let rd = read(fd as u64, &mut file_data);
    close(fd as u64);
    let actual_len = (rd as usize).min(uncompressed_size as usize);
    let payload = &file_data[..actual_len];

    let mut hasher = Crc32::new();
    hasher.update(payload);
    let crc = hasher.finish();

    let mut comp_buf = [0u8; 65536];
    let mut comp_len = 0usize;

    let use_deflate = !store_only && actual_len > 16;
    let method = if use_deflate {
        let res = libwunix::deflate_fixed(payload, |chunk| {
            if comp_len + chunk.len() <= comp_buf.len() {
                comp_buf[comp_len..comp_len + chunk.len()].copy_from_slice(chunk);
                comp_len += chunk.len();
                Ok(())
            } else {
                Err(libwunix::CompressionError::OutputBufferTooSmall)
            }
        });
        if res.is_ok() && comp_len < actual_len {
            METHOD_DEFLATE
        } else {
            METHOD_STORE
        }
    } else {
        METHOD_STORE
    };

    let (final_payload, compressed_size) = if method == METHOD_DEFLATE {
        (&comp_buf[..comp_len], comp_len as u32)
    } else {
        (payload, actual_len as u32)
    };

    // Записываем Local File Header
    let mut local_hdr = [0u8; LOCAL_HEADER_SIZE];
    write_local_header(
        &mut local_hdr,
        method,
        crc,
        compressed_size,
        uncompressed_size,
        name_len as u16,
    );
    write(out_fd, &local_hdr);
    write(out_fd, &zip_name[..z_len]);
    write(out_fd, final_payload);

    state.current_offset += (LOCAL_HEADER_SIZE + name_len + final_payload.len()) as u32;

    state.entries[state.entry_count] = ZipEntryMeta {
        name_offset,
        name_len,
        method,
        crc32: crc,
        compressed_size,
        uncompressed_size,
        local_header_offset,
        mode,
        is_dir: false,
    };
    state.entry_count += 1;

    if verbose {
        write(STDOUT_FILENO, b"  adding: ");
        write(STDOUT_FILENO, &zip_name[..z_len]);
        if method == METHOD_DEFLATE {
            write(STDOUT_FILENO, b" (deflated)\n");
        } else {
            write(STDOUT_FILENO, b" (stored)\n");
        }
    }

    Ok(())
}

pub fn finalize_zip(out_fd: u64, state: &ZipArchiveState) -> Result<(), ()> {
    let cd_offset = state.current_offset;
    let mut cd_size = 0u32;

    for i in 0..state.entry_count {
        let entry = &state.entries[i];
        let mut cd_hdr = [0u8; CENTRAL_HEADER_SIZE];
        write_central_header(&mut cd_hdr, entry);
        write(out_fd, &cd_hdr);

        let name = &state.name_storage[entry.name_offset..entry.name_offset + entry.name_len];
        write(out_fd, name);

        cd_size += (CENTRAL_HEADER_SIZE + entry.name_len) as u32;
    }

    let mut eocd = [0u8; EOCD_SIZE];
    write_eocd(&mut eocd, state.entry_count as u16, cd_size, cd_offset);
    write(out_fd, &eocd);

    Ok(())
}
