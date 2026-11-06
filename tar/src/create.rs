// Создание архива tar (-c).
// Рекурсивный обход каталогов, чтение атрибутов через lstat,
// формирование ustar заголовков и выравнивание блоков по 512 байт.

use libwunix::{
    close, getdents64, lstat, open, read, readlink, write, Stat, O_RDONLY,
    STDOUT_FILENO,
};
use crate::header::{
    TarEntryHeader, BLOCK_SIZE, TYPE_DIRECTORY, TYPE_REGULAR, TYPE_SYMLINK,
};

pub fn append_file_content(out_fd: u64, file_fd: u64, size: u64) -> Result<(), ()> {
    let mut remaining = size;
    let mut block = [0u8; BLOCK_SIZE];

    while remaining > 0 {
        let chunk = (remaining as usize).min(BLOCK_SIZE);
        let rd = read(file_fd, &mut block[..chunk]);
        if rd == 0 {
            break;
        }
        // Дополняем последний блок нулями
        if (rd as usize) < BLOCK_SIZE {
            block[rd as usize..].fill(0);
        }
        let wr = write(out_fd, &block);
        if wr < BLOCK_SIZE as u64 {
            return Err(());
        }
        remaining = remaining.saturating_sub(rd);
    }

    Ok(())
}

pub fn add_path(out_fd: u64, path: &[u8], verbose: bool) -> Result<(), ()> {
    let mut path_nt = [0u8; 513];
    if path.len() >= 512 {
        return Err(());
    }
    path_nt[..path.len()].copy_from_slice(path);
    path_nt[path.len()] = 0;
    let path_slice = &path_nt[..path.len() + 1];

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
    let is_symlink = (st.st_mode & 0o170000) == 0o120000;

    let mut hdr = TarEntryHeader::default();
    let name_len = path.len().min(99);
    hdr.name[..name_len].copy_from_slice(&path[..name_len]);

    hdr.mode = (st.st_mode & 0o7777) as u32;
    hdr.uid = st.st_uid;
    hdr.gid = st.st_gid;
    hdr.mtime = st.st_mtime as u64;

    if is_dir {
        hdr.typeflag = TYPE_DIRECTORY;
        hdr.size = 0;
        if name_len < 99 && !hdr.name.starts_with(b"/") && hdr.name[name_len - 1] != b'/' {
            hdr.name[name_len] = b'/';
        }
    } else if is_symlink {
        hdr.typeflag = TYPE_SYMLINK;
        hdr.size = 0;
        let mut target_buf = [0u8; 100];
        let link_len = readlink(path_slice, &mut target_buf);
        if link_len > 0 {
            hdr.linkname[..link_len as usize].copy_from_slice(&target_buf[..link_len as usize]);
        }
    } else {
        hdr.typeflag = TYPE_REGULAR;
        hdr.size = st.st_size as u64;
    }

    let mut block = [0u8; BLOCK_SIZE];
    hdr.serialize(&mut block);
    if write(out_fd, &block) < BLOCK_SIZE as u64 {
        return Err(());
    }

    if verbose {
        write(STDOUT_FILENO, path);
        write(STDOUT_FILENO, b"\n");
    }

    if !is_dir && !is_symlink && hdr.size > 0 {
        let fd = open(path_slice, O_RDONLY as u64, 0);
        if fd >= 0 {
            let res = append_file_content(out_fd, fd as u64, hdr.size);
            close(fd as u64);
            res?;
        }
    }

    if is_dir {
        // Рекурсивно обходим директорию
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
                        let mut p_len = path.len();
                        subpath[..p_len].copy_from_slice(path);
                        if subpath[p_len - 1] != b'/' {
                            subpath[p_len] = b'/';
                            p_len += 1;
                        }
                        let c_len = child_name.len().min(512 - p_len);
                        subpath[p_len..p_len + c_len].copy_from_slice(&child_name[..c_len]);
                        p_len += c_len;

                        let _ = add_path(out_fd, &subpath[..p_len], verbose);
                    }

                    offset += d_reclen;
                }
            }
            close(dir_fd as u64);
        }
    }

    Ok(())
}

pub fn finalize_archive(out_fd: u64) -> Result<(), ()> {
    // По стандарту ustar архив завершается двумя нулевыми 512-байтными блоками
    let zero_blocks = [0u8; BLOCK_SIZE * 2];
    if write(out_fd, &zero_blocks) < zero_blocks.len() as u64 {
        return Err(());
    }
    Ok(())
}
