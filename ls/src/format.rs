use libwunix::{readlink, write, STDOUT_FILENO};

pub const DT_DIR: u8 = 4;
pub const DT_REG: u8 = 8;
pub const DT_LNK: u8 = 10;

// ANSI escape sequences
pub const ANSI_RESET: &[u8] = b"\x1b[0m";
pub const ANSI_DIR: &[u8] = b"\x1b[1;34m";    // Жирный синий для директорий
pub const ANSI_LNK: &[u8] = b"\x1b[1;36m";    // Жирный бирюзовый (cyan) для симлинков
pub const ANSI_EXEC: &[u8] = b"\x1b[1;32m";   // Зеленый для исполняемых файлов

pub fn is_executable(name: &str, mode: u32) -> bool {
    name.ends_with(".exe") || (mode & 0o111) != 0
}

pub fn print_colored_name(name: &str, dtype: u8, is_exec: bool) {
    if dtype == DT_DIR {
        write(STDOUT_FILENO, ANSI_DIR);
        write(STDOUT_FILENO, name.as_bytes());
        write(STDOUT_FILENO, ANSI_RESET);
    } else if dtype == DT_LNK {
        write(STDOUT_FILENO, ANSI_LNK);
        write(STDOUT_FILENO, name.as_bytes());
        write(STDOUT_FILENO, ANSI_RESET);
    } else if is_exec {
        write(STDOUT_FILENO, ANSI_EXEC);
        write(STDOUT_FILENO, name.as_bytes());
        write(STDOUT_FILENO, ANSI_RESET);
    } else {
        write(STDOUT_FILENO, name.as_bytes());
    }
}

pub fn format_mode_string(dtype: u8, mode: u32, out: &mut [u8; 10]) {
    out[0] = match dtype {
        DT_DIR => b'd',
        DT_LNK => b'l',
        _ => b'-',
    };
    out[1] = if (mode & 0o400) != 0 { b'r' } else { b'-' };
    out[2] = if (mode & 0o200) != 0 { b'w' } else { b'-' };
    out[3] = if (mode & 0o4000) != 0 {
        if (mode & 0o100) != 0 { b's' } else { b'S' }
    } else if (mode & 0o100) != 0 {
        b'x'
    } else {
        b'-'
    };

    out[4] = if (mode & 0o040) != 0 { b'r' } else { b'-' };
    out[5] = if (mode & 0o020) != 0 { b'w' } else { b'-' };
    out[6] = if (mode & 0o2000) != 0 {
        if (mode & 0o010) != 0 { b's' } else { b'S' }
    } else if (mode & 0o010) != 0 {
        b'x'
    } else {
        b'-'
    };

    out[7] = if (mode & 0o004) != 0 { b'r' } else { b'-' };
    out[8] = if (mode & 0o002) != 0 { b'w' } else { b'-' };
    out[9] = if (mode & 0o1000) != 0 {
        if (mode & 0o001) != 0 { b't' } else { b'T' }
    } else if (mode & 0o001) != 0 {
        b'x'
    } else {
        b'-'
    };
}

pub fn print_long_entry(base_dir: &str, name: &str, dtype: u8, size: u64, mode: u32, uid: u32, gid: u32) {
    let mut mode_buf = [0u8; 10];
    format_mode_string(dtype, mode, &mut mode_buf);
    write(STDOUT_FILENO, &mode_buf);

    let mut u_buf = [0u8; 16];
    let mut g_buf = [0u8; 16];
    let pwd_opt = libwunix::get_passwd_by_uid(uid).ok().flatten();
    let grp_opt = libwunix::get_group_by_gid(gid).ok().flatten();

    let uname = match &pwd_opt {
        Some(pwd) => pwd.username(),
        None => libwunix::fmt::utoa(uid as u64, &mut u_buf),
    };
    let gname = match &grp_opt {
        Some(grp) => grp.name(),
        None => libwunix::fmt::utoa(gid as u64, &mut g_buf),
    };

    write(STDOUT_FILENO, b"  1 ");
    write(STDOUT_FILENO, uname.as_bytes());
    write(STDOUT_FILENO, b" ");
    write(STDOUT_FILENO, gname.as_bytes());
    write(STDOUT_FILENO, b" ");

    // Форматируем размер с выравниванием по правому краю (8 символов)
    let mut size_buf = [0u8; 20];
    let mut val = size;
    let mut s_len = 0;
    if val == 0 {
        size_buf[0] = b'0';
        s_len = 1;
    } else {
        while val > 0 {
            size_buf[s_len] = b'0' + (val % 10) as u8;
            val /= 10;
            s_len += 1;
        }
    }
    let spaces = if s_len < 8 { 8 - s_len } else { 0 };
    for _ in 0..spaces {
        write(STDOUT_FILENO, b" ");
    }
    for j in 0..s_len {
        let b = [size_buf[s_len - 1 - j]];
        write(STDOUT_FILENO, &b);
    }
    write(STDOUT_FILENO, b" ");

    let is_exec = is_executable(name, mode);
    print_colored_name(name, dtype, is_exec);

    if dtype == DT_LNK {
        // Читаем цель символической ссылки
        let mut full_path = [0u8; 1024];
        let mut f_len = 0;

        let b_bytes = base_dir.as_bytes();
        if base_dir != "." {
            let copy_b = b_bytes.len().min(full_path.len() - 1);
            full_path[..copy_b].copy_from_slice(&b_bytes[..copy_b]);
            f_len += copy_b;
            if !base_dir.ends_with('/') && f_len < full_path.len() - 1 {
                full_path[f_len] = b'/';
                f_len += 1;
            }
        }
        let n_bytes = name.as_bytes();
        let copy_n = n_bytes.len().min(full_path.len().saturating_sub(f_len) - 1);
        full_path[f_len..f_len + copy_n].copy_from_slice(&n_bytes[..copy_n]);
        f_len += copy_n;
        full_path[f_len] = 0;

        let mut target_buf = [0u8; 512];
        let ret = readlink(&full_path[..f_len + 1], &mut target_buf);
        if ret > 0 {
            write(STDOUT_FILENO, b" -> ");
            write(STDOUT_FILENO, &target_buf[..ret as usize]);
        }
    }

    write(STDOUT_FILENO, b"\n");
}
