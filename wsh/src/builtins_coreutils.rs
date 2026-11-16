#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use crate::builtins::{print_err, print_out};
use crate::platform;

pub fn builtin_date(args: &[String]) -> i32 {
    let mut is_utc = false;
    let mut custom_fmt: Option<&str> = None;
    let mut rfc_email = false;
    let mut iso_8601 = false;

    for arg in args {
        if arg == "-h" || arg == "--help" {
            print_out("Usage: date [OPTION]... [+FORMAT]\n");
            return 0;
        } else if arg == "-V" || arg == "--version" {
            print_out("date (Wunix built-in) 0.1.0\n");
            return 0;
        } else if arg == "-u" || arg == "--utc" || arg == "--universal" {
            is_utc = true;
        } else if arg == "-R" || arg == "--rfc-email" {
            rfc_email = true;
        } else if arg == "-I" || arg == "--iso-8601" {
            iso_8601 = true;
        } else if let Some(fmt) = arg.strip_prefix('+') {
            custom_fmt = Some(fmt);
        }
    }

    #[cfg(not(feature = "std"))]
    let (tz_offset, tz_name, _) = if is_utc {
        (0i32, "UTC", "UTC")
    } else {
        libwunix::time::get_system_timezone()
    };
    #[cfg(feature = "std")]
    let (tz_offset, tz_name) = if is_utc {
        (0i32, "UTC")
    } else {
        (10800i32, "MSK")
    };

    #[cfg(not(feature = "std"))]
    let now_ts = libwunix::time::clock_realtime();
    #[cfg(feature = "std")]
    let now_ts = {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        libwunix::time::Timespec {
            tv_sec: dur.as_secs() as i64,
            tv_nsec: dur.subsec_nanos() as i64,
        }
    };

    let dt = libwunix::time::DateTime::from_epoch(now_ts.tv_sec, now_ts.tv_nsec as u32, tz_offset);
    let mut buf = [0u8; 256];

    #[cfg(not(feature = "std"))]
    let is_ru = libwunix::current_language() == libwunix::Language::Russian;
    #[cfg(feature = "std")]
    let is_ru = false;

    if let Some(fmt) = custom_fmt {
        let s = dt.format_strftime(fmt, tz_name, tz_offset, is_ru, &mut buf);
        print_out(s);
        print_out("\n");
    } else if rfc_email {
        let s = dt.format_rfc5322(tz_offset, &mut buf);
        print_out(s);
        print_out("\n");
    } else if iso_8601 {
        let s = dt.format_iso8601(false, tz_offset, &mut buf);
        print_out(s);
        print_out("\n");
    } else {
        let s = dt.format_default(tz_name, is_ru, &mut buf);
        print_out(s);
        print_out("\n");
    }

    0
}

pub fn builtin_id(args: &[String]) -> i32 {
    let mut only_user = false;
    let mut only_group = false;
    let mut only_groups = false;
    let mut name = false;
    let mut real = false;
    let mut specified_user: Option<&str> = None;

    for arg in args {
        if arg == "-h" || arg == "--help" {
            print_out("Usage: id [OPTION]... [USER]\n");
            return 0;
        } else if arg == "-u" || arg == "--user" {
            only_user = true;
        } else if arg == "-g" || arg == "--group" {
            only_group = true;
        } else if arg == "-G" || arg == "--groups" {
            only_groups = true;
        } else if arg == "-n" || arg == "--name" {
            name = true;
        } else if arg == "-r" || arg == "--real" {
            real = true;
        } else if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") {
            for b in arg.bytes().skip(1) {
                match b {
                    b'u' => only_user = true,
                    b'g' => only_group = true,
                    b'G' => only_groups = true,
                    b'n' => name = true,
                    b'r' => real = true,
                    _ => {}
                }
            }
        } else if specified_user.is_none() {
            specified_user = Some(arg.as_str());
        }
    }

    let default_user = platform::get_env("USER").unwrap_or_else(|| String::from("root"));
    let target_user = specified_user.unwrap_or(&default_user);

    #[cfg(not(feature = "std"))]
    let (uid, gid) = if let Some(u) = specified_user {
        match libwunix::get_passwd_by_name(u) {
            Ok(Some(entry)) => (entry.uid, entry.gid),
            _ => (0, 0),
        }
    } else {
        let u = if real { libwunix::getuid() } else { libwunix::geteuid() };
        let g = if real { libwunix::getgid() } else { libwunix::getegid() };
        (u, g)
    };
    #[cfg(feature = "std")]
    let (uid, gid) = (0u32, 0u32);

    if only_user {
        if name {
            print_out(target_user);
        } else {
            let mut num_buf = [0u8; 16];
            let s = libwunix::fmt::utoa(uid as u64, &mut num_buf);
            print_out(s);
        }
        print_out("\n");
        return 0;
    }

    if only_group || only_groups {
        if name {
            print_out(target_user);
        } else {
            let mut num_buf = [0u8; 16];
            let s = libwunix::fmt::utoa(gid as u64, &mut num_buf);
            print_out(s);
        }
        print_out("\n");
        return 0;
    }

    let mut num_buf_u = [0u8; 16];
    let mut num_buf_g = [0u8; 16];
    let u_str = libwunix::fmt::utoa(uid as u64, &mut num_buf_u);
    let g_str = libwunix::fmt::utoa(gid as u64, &mut num_buf_g);

    print_out("uid=");
    print_out(u_str);
    print_out("(");
    print_out(target_user);
    print_out(") gid=");
    print_out(g_str);
    print_out("(");
    print_out(target_user);
    print_out(") groups=");
    print_out(g_str);
    print_out("(");
    print_out(target_user);
    print_out(")\n");
    0
}

#[derive(Default, Clone, Copy)]
struct WcCounts {
    lines: u64,
    words: u64,
    bytes: u64,
    chars: u64,
    max_line_len: u64,
}

#[derive(Clone, Copy)]
struct WcOptions {
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
    max_line_len: bool,
}

impl WcOptions {
    fn any_set(self) -> bool {
        self.lines || self.words || self.bytes || self.chars || self.max_line_len
    }

    fn count_selected(self) -> usize {
        self.lines as usize
            + self.words as usize
            + self.bytes as usize
            + self.chars as usize
            + self.max_line_len as usize
    }
}

struct WcCounterState {
    counts: WcCounts,
    in_word: bool,
    cur_line_len: u64,
}

impl WcCounterState {
    fn new() -> Self {
        Self {
            counts: WcCounts::default(),
            in_word: false,
            cur_line_len: 0,
        }
    }

    fn feed(&mut self, data: &[u8]) {
        self.counts.bytes += data.len() as u64;
        for &b in data {
            if (b & 0xC0) != 0x80 {
                self.counts.chars += 1;
            }
            if b == b'\n' {
                self.counts.lines += 1;
                if self.cur_line_len > self.counts.max_line_len {
                    self.counts.max_line_len = self.cur_line_len;
                }
                self.cur_line_len = 0;
            } else if (b & 0xC0) != 0x80 {
                self.cur_line_len += 1;
            }

            if b.is_ascii_whitespace() {
                self.in_word = false;
            } else if !self.in_word {
                self.in_word = true;
                self.counts.words += 1;
            }
        }
    }

    fn finish(mut self) -> WcCounts {
        if self.cur_line_len > self.counts.max_line_len {
            self.counts.max_line_len = self.cur_line_len;
        }
        self.counts
    }
}

fn count_fd(fd: u64) -> WcCounts {
    let mut state = WcCounterState::new();
    let mut buf = [0u8; 4096];
    #[cfg(not(feature = "std"))]
    loop {
        let n = libwunix::read(fd, &mut buf);
        if n <= 0 {
            break;
        }
        state.feed(&buf[..n as usize]);
    }
    #[cfg(feature = "std")]
    loop {
        let n = unsafe { libc::read(fd as i32, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        state.feed(&buf[..n as usize]);
    }
    state.finish()
}

fn print_counts(counts: WcCounts, opts: WcOptions, name: Option<&str>) {
    let mut num_buf = [0u8; 24];
    // Для одиночного вывода без имени файла не делаем фиксированный отступ шириной 7 символов
    let single = opts.count_selected() == 1 && name.is_none();
    let mut first = true;

    let mut emit = |val: u64| {
        if !first {
            print_out(" ");
        }
        first = false;
        let s = libwunix::fmt::utoa(val, &mut num_buf);
        if !single && s.len() < 7 {
            let pad = 7 - s.len();
            for _ in 0..pad {
                print_out(" ");
            }
        }
        print_out(s);
    };

    if opts.lines {
        emit(counts.lines);
    }
    if opts.words {
        emit(counts.words);
    }
    if opts.chars {
        emit(counts.chars);
    }
    if opts.bytes {
        emit(counts.bytes);
    }
    if opts.max_line_len {
        emit(counts.max_line_len);
    }
    if let Some(n) = name {
        print_out(" ");
        print_out(n);
    }
    print_out("\n");
}

pub fn builtin_wc(args: &[String]) -> i32 {
    let mut opts = WcOptions {
        lines: false,
        words: false,
        bytes: false,
        chars: false,
        max_line_len: false,
    };
    let mut files = Vec::new();

    for arg in args {
        if arg == "-h" || arg == "--help" {
            print_out("Usage: wc [OPTION]... [FILE]...\n");
            return 0;
        } else if arg == "-c" || arg == "--bytes" {
            opts.bytes = true;
        } else if arg == "-m" || arg == "--chars" {
            opts.chars = true;
        } else if arg == "-l" || arg == "--lines" {
            opts.lines = true;
        } else if arg == "-w" || arg == "--words" {
            opts.words = true;
        } else if arg == "-L" || arg == "--max-line-length" {
            opts.max_line_len = true;
        } else if arg.starts_with('-') && arg.len() > 1 && !arg.starts_with("--") {
            for b in arg.bytes().skip(1) {
                match b {
                    b'c' => opts.bytes = true,
                    b'm' => opts.chars = true,
                    b'l' => opts.lines = true,
                    b'w' => opts.words = true,
                    b'L' => opts.max_line_len = true,
                    _ => {}
                }
            }
        } else {
            files.push(arg.as_str());
        }
    }

    if !opts.any_set() {
        opts.lines = true;
        opts.words = true;
        opts.bytes = true;
    }

    if files.is_empty() {
        let c = count_fd(0);
        print_counts(c, opts, None);
        return 0;
    }

    let mut total = WcCounts::default();
    let mut exit_code = 0;

    for &path in &files {
        if path == "-" {
            let c = count_fd(0);
            print_counts(c, opts, Some("-"));
            total.lines += c.lines;
            total.words += c.words;
            total.bytes += c.bytes;
            total.chars += c.chars;
            if c.max_line_len > total.max_line_len {
                total.max_line_len = c.max_line_len;
            }
            continue;
        }

        #[cfg(not(feature = "std"))]
        let fd = {
            let mut null_buf = [0u8; 512];
            let b = path.as_bytes();
            if b.len() >= null_buf.len() {
                -1
            } else {
                null_buf[..b.len()].copy_from_slice(b);
                null_buf[b.len()] = 0;
                libwunix::open(&null_buf[..b.len() + 1], libwunix::O_RDONLY as u64, 0)
            }
        };
        #[cfg(feature = "std")]
        let fd = {
            if let Ok(c) = std::ffi::CString::new(path) {
                unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) as i64 }
            } else {
                -1
            }
        };

        if fd < 0 {
            print_err("wc: ");
            print_err(path);
            print_err(": No such file or directory\n");
            exit_code = 1;
            continue;
        }

        let c = count_fd(fd as u64);
        #[cfg(not(feature = "std"))]
        let _ = libwunix::close(fd as u64);
        #[cfg(feature = "std")]
        unsafe {
            libc::close(fd as i32);
        }

        print_counts(c, opts, Some(path));
        total.lines += c.lines;
        total.words += c.words;
        total.bytes += c.bytes;
        total.chars += c.chars;
        if c.max_line_len > total.max_line_len {
            total.max_line_len = c.max_line_len;
        }
    }

    if files.len() > 1 {
        print_counts(total, opts, Some("total"));
    }

    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_date_help_and_version() {
        assert_eq!(builtin_date(&[String::from("--help")]), 0);
        assert_eq!(builtin_date(&[String::from("--version")]), 0);
    }

    #[test]
    fn test_builtin_id_user_and_name() {
        assert_eq!(builtin_id(&[String::from("-u")]), 0);
        assert_eq!(builtin_id(&[String::from("-un")]), 0);
    }

    #[test]
    fn test_wc_state_feed() {
        let mut state = WcCounterState::new();
        state.feed(b"hello world\nsecond line\n");
        let counts = state.finish();
        assert_eq!(counts.lines, 2);
        assert_eq!(counts.words, 4);
        assert_eq!(counts.bytes, 24);
    }
}
