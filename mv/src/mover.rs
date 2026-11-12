use crate::cli::{Options, OverwriteMode};
use crate::io::{print_err, print_move_error, print_out, prompt_overwrite};
use crate::path::{
    basename, is_dir_mode, is_lnk_mode, is_reg_mode, join_path, to_cstr, MAX_PATH,
};
use libwunix::{
    chmod, chown, close, getdents64, lstat, mkdir, open, read, readlink, renameat2, rmdir, stat,
    symlink, t, unlink, write, AT_FDCWD, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY, RENAME_NOREPLACE,
    STDOUT_FILENO, Stat,
};

const EXDEV: i64 = -18;
const ENOTEMPTY: i64 = -39;
const ENOTDIR: i64 = -20;
const EISDIR: i64 = -21;
const ENAMETOOLONG: i64 = -36;
const COPY_BUF_SIZE: usize = 4096;

pub fn is_existing_dir(path: &[u8]) -> bool {
    let mut buf = [0u8; MAX_PATH];
    let Some(cstr) = to_cstr(path, &mut buf) else {
        return false;
    };
    let mut st = Stat::zeroed();
    stat(cstr, &mut st) == 0 && is_dir_mode(st.st_mode)
}

pub fn move_into_dir(src: &[u8], dir: &[u8], opts: Options) -> bool {
    let name = basename(src);
    let mut joined = [0u8; MAX_PATH];
    let Some(dst) = join_path(dir, name, &mut joined) else {
        print_err(t("mv: путь слишком длинный\n", "mv: path too long\n"));
        return false;
    };
    move_entry(src, dst, opts)
}

pub fn move_entry(src: &[u8], dst: &[u8], opts: Options) -> bool {
    let mut src_buf = [0u8; MAX_PATH];
    let mut dst_buf = [0u8; MAX_PATH];
    let (Some(src_c), Some(dst_c)) = (to_cstr(src, &mut src_buf), to_cstr(dst, &mut dst_buf)) else {
        print_move_error(src, dst, ENAMETOOLONG);
        return false;
    };

    let mut src_st = Stat::zeroed();
    let r = lstat(src_c, &mut src_st);
    if r < 0 {
        print_move_error(src, dst, r);
        return false;
    }

    let mut dst_st = Stat::zeroed();
    let dst_exists = lstat(dst_c, &mut dst_st) == 0;

    if dst_exists && !should_overwrite(dst, &src_st, &dst_st, opts) {
        return true;
    }

    let flags = if opts.overwrite == OverwriteMode::NoClobber {
        RENAME_NOREPLACE
    } else {
        0
    };

    let ret = renameat2(AT_FDCWD, src_c, AT_FDCWD, dst_c, flags);
    if ret == 0 {
        print_verbose(src, dst, opts.verbose);
        return true;
    }

    if ret == EXDEV {
        let res = cross_device_move(src, dst, src_c, dst_c, &src_st, dst_exists, &dst_st, opts);
        if res == 0 {
            print_verbose(src, dst, opts.verbose);
            return true;
        }
        print_move_error(src, dst, res);
        return false;
    }

    print_move_error(src, dst, ret);
    false
}

fn should_overwrite(dst: &[u8], src_st: &Stat, dst_st: &Stat, opts: Options) -> bool {
    if opts.overwrite == OverwriteMode::NoClobber {
        return false;
    }
    if opts.update && !is_newer(src_st, dst_st) {
        return false;
    }
    if opts.overwrite == OverwriteMode::Interactive {
        return prompt_overwrite(dst);
    }
    true
}

fn is_newer(src: &Stat, dst: &Stat) -> bool {
    (src.st_mtime, src.st_mtime_nsec) > (dst.st_mtime, dst.st_mtime_nsec)
}

fn print_verbose(src: &[u8], dst: &[u8], verbose: bool) {
    if !verbose {
        return;
    }
    print_out(t("переименован '", "renamed '"));
    write(STDOUT_FILENO, src);
    print_out("' -> '");
    write(STDOUT_FILENO, dst);
    print_out("'\n");
}

#[allow(clippy::too_many_arguments)]
fn cross_device_move(
    src: &[u8],
    dst: &[u8],
    src_c: &[u8],
    dst_c: &[u8],
    src_st: &Stat,
    dst_exists: bool,
    dst_st: &Stat,
    opts: Options,
) -> i64 {
    if is_lnk_mode(src_st.st_mode) {
        return move_symlink_cross_dev(src_c, dst_c, dst_exists);
    }
    if is_reg_mode(src_st.st_mode) {
        if dst_exists && is_dir_mode(dst_st.st_mode) {
            return EISDIR;
        }
        return move_file_cross_dev(src_c, dst_c, src_st);
    }
    if is_dir_mode(src_st.st_mode) {
        if dst_exists && !is_dir_mode(dst_st.st_mode) {
            return ENOTDIR;
        }
        return move_dir_cross_dev(src, dst, src_c, dst_c, src_st, dst_exists, opts);
    }
    -1
}

fn move_symlink_cross_dev(src_c: &[u8], dst_c: &[u8], dst_exists: bool) -> i64 {
    let mut target = [0u8; MAX_PATH];
    let len = readlink(src_c, &mut target[..MAX_PATH - 1]);
    if len < 0 {
        return len;
    }
    let len = len as usize;
    target[len] = 0;

    if dst_exists {
        let u = unlink(dst_c);
        if u < 0 {
            return u;
        }
    }
    let s = symlink(&target[..len + 1], dst_c);
    if s < 0 {
        return s;
    }
    unlink(src_c)
}

fn move_file_cross_dev(src_c: &[u8], dst_c: &[u8], src_st: &Stat) -> i64 {
    let in_fd = open(src_c, O_RDONLY as u64, 0);
    if in_fd < 0 {
        return in_fd;
    }
    let mode = src_st.st_mode & 0o7777;
    let out_fd = open(dst_c, (O_WRONLY | O_CREAT | O_TRUNC) as u64, mode as u64);
    if out_fd < 0 {
        let _ = close(in_fd as u64);
        return out_fd;
    }

    let copy_res = copy_fd_contents(in_fd as u64, out_fd as u64);
    let _ = close(in_fd as u64);
    let _ = close(out_fd as u64);
    if copy_res < 0 {
        let _ = unlink(dst_c);
        return copy_res;
    }

    let _ = chmod(dst_c, mode);
    let _ = chown(dst_c, src_st.st_uid, src_st.st_gid);
    unlink(src_c)
}

fn copy_fd_contents(in_fd: u64, out_fd: u64) -> i64 {
    let mut buf = [0u8; COPY_BUF_SIZE];
    loop {
        let n = read(in_fd, &mut buf) as i64;
        if n < 0 {
            return n;
        }
        if n == 0 {
            return 0;
        }
        let mut written = 0usize;
        let total = n as usize;
        while written < total {
            let w = write(out_fd, &buf[written..total]) as i64;
            if w <= 0 {
                return if w < 0 { w } else { -5 };
            }
            written += w as usize;
        }
    }
}

fn move_dir_cross_dev(
    src: &[u8],
    dst: &[u8],
    src_c: &[u8],
    dst_c: &[u8],
    src_st: &Stat,
    dst_exists: bool,
    opts: Options,
) -> i64 {
    let mode = src_st.st_mode & 0o7777;
    if dst_exists {
        if !is_dir_empty(dst_c) {
            return ENOTEMPTY;
        }
    } else {
        let m = mkdir(dst_c, mode);
        if m < 0 {
            return m;
        }
    }

    let mut child_src_buf = [0u8; MAX_PATH];
    let mut child_dst_buf = [0u8; MAX_PATH];
    let mut inner_err = 0i64;

    let scan_res = for_each_dirent(src_c, |name| {
        let (Some(cs), Some(cd)) = (
            join_path(src, name, &mut child_src_buf),
            join_path(dst, name, &mut child_dst_buf),
        ) else {
            inner_err = ENAMETOOLONG;
            return false;
        };
        if !move_entry(cs, cd, opts) {
            inner_err = -1;
            return false;
        }
        true
    });

    if scan_res < 0 {
        return scan_res;
    }
    if inner_err != 0 {
        return inner_err;
    }

    let _ = chmod(dst_c, mode);
    let _ = chown(dst_c, src_st.st_uid, src_st.st_gid);
    rmdir(src_c)
}

fn is_dir_empty(dir_c: &[u8]) -> bool {
    let mut empty = true;
    let res = for_each_dirent(dir_c, |_| {
        empty = false;
        false
    });
    res == 0 && empty
}

fn for_each_dirent<F: FnMut(&[u8]) -> bool>(dir_c: &[u8], mut cb: F) -> i64 {
    let fd = open(dir_c, O_RDONLY as u64, 0);
    if fd < 0 {
        return fd;
    }
    let mut buf = [0u8; 2048];
    loop {
        let n = getdents64(fd as u64, &mut buf);
        if n <= 0 {
            break;
        }
        let total = (n as usize).min(buf.len());
        let mut pos = 0usize;
        while pos + 19 <= total {
            let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
            if reclen == 0 || pos + reclen > total {
                break;
            }
            let name = extract_dirent_name(&buf[pos + 19..pos + reclen]);
            pos += reclen;
            if name.is_empty() || name == b"." || name == b".." {
                continue;
            }
            if !cb(name) {
                let _ = close(fd as u64);
                return 0;
            }
        }
    }
    let _ = close(fd as u64);
    0
}

fn extract_dirent_name(slice: &[u8]) -> &[u8] {
    let mut end = 0usize;
    while end < slice.len() && slice[end] != 0 {
        end += 1;
    }
    &slice[..end]
}
