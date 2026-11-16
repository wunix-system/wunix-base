#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
use libwunix::{close, exit, open, read, str_utils, t, write, O_RDONLY, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};

#[derive(Default, Clone, Copy)]
struct Counts {
    lines: u64,
    words: u64,
    bytes: u64,
    chars: u64,
    max_line_len: u64,
}

#[derive(Clone, Copy)]
struct Options {
    lines: bool,
    words: bool,
    bytes: bool,
    chars: bool,
    max_line_len: bool,
}

impl Options {
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

struct CounterState {
    counts: Counts,
    in_word: bool,
    cur_line_len: u64,
}

impl CounterState {
    fn new() -> Self {
        Self {
            counts: Counts::default(),
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

    fn finish(mut self) -> Counts {
        if self.cur_line_len > self.counts.max_line_len {
            self.counts.max_line_len = self.cur_line_len;
        }
        self.counts
    }
}

#[cfg(not(test))]
fn count_fd(fd: u64) -> Counts {
    let mut state = CounterState::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = read(fd, &mut buf);
        if n <= 0 {
            break;
        }
        state.feed(&buf[..n as usize]);
    }
    state.finish()
}

#[cfg(not(test))]
fn print_counts(counts: Counts, opts: Options, name: Option<&str>) {
    let mut num_buf = [0u8; 24];
    let single = opts.count_selected() == 1 && name.is_none();
    let mut first = true;

    let emit = |val: u64, first: &mut bool, num_buf: &mut [u8; 24]| {
        if !*first {
            write(STDOUT_FILENO, b" ");
        }
        *first = false;
        let s = libwunix::fmt::utoa(val, num_buf);
        if !single && s.len() < 7 {
            let pad = 7 - s.len();
            write(STDOUT_FILENO, &b"       "[..pad]);
        }
        write(STDOUT_FILENO, s.as_bytes());
    };

    if opts.lines {
        emit(counts.lines, &mut first, &mut num_buf);
    }
    if opts.words {
        emit(counts.words, &mut first, &mut num_buf);
    }
    if opts.chars {
        emit(counts.chars, &mut first, &mut num_buf);
    }
    if opts.bytes {
        emit(counts.bytes, &mut first, &mut num_buf);
    }
    if opts.max_line_len {
        emit(counts.max_line_len, &mut first, &mut num_buf);
    }
    if let Some(n) = name {
        write(STDOUT_FILENO, b" ");
        write(STDOUT_FILENO, n.as_bytes());
    }
    write(STDOUT_FILENO, b"\n");
}

#[cfg(not(test))]
fn print_help() {
    let msg = t(
        "Использование: wc [ОПЦИЯ]... [ФАЙЛ]...\n\
         Подсчитывает строки, слова и байты для каждого ФАЙЛА или стандартного ввода.\n\n\
         Опции:\n\
           -c, --bytes            вывести количество байт\n\
           -m, --chars            вывести количество символов\n\
           -l, --lines            вывести количество строк\n\
           -L, --max-line-length  вывести длину самой длинной строки\n\
           -w, --words            вывести количество слов\n\
           -h, --help             показать эту справку и выйти\n",
        "Usage: wc [OPTION]... [FILE]...\n\
         Print newline, word, and byte counts for each FILE, or standard input.\n\n\
         Options:\n\
           -c, --bytes            print the byte counts\n\
           -m, --chars            print the character counts\n\
           -l, --lines            print the newline counts\n\
           -L, --max-line-length  print the maximum display width\n\
           -w, --words            print the word counts\n\
           -h, --help             display this help and exit\n",
    );
    write(STDOUT_FILENO, msg.as_bytes());
}

#[cfg(not(test))]
#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut opts = Options {
        lines: false,
        words: false,
        bytes: false,
        chars: false,
        max_line_len: false,
    };
    let mut files: [&str; 64] = [""; 64];
    let mut file_count = 0usize;

    for i in 1..argc {
        let p = unsafe { *argv.add(i as usize) };
        if p.is_null() {
            continue;
        }
        let len = str_utils::strlen(p);
        let arg = core::str::from_utf8(unsafe { core::slice::from_raw_parts(p, len) }).unwrap_or("");
        if arg == "--help" || arg == "-h" {
            print_help();
            exit(0);
        } else if arg == "--bytes" {
            opts.bytes = true;
        } else if arg == "--chars" {
            opts.chars = true;
        } else if arg == "--lines" {
            opts.lines = true;
        } else if arg == "--words" {
            opts.words = true;
        } else if arg == "--max-line-length" {
            opts.max_line_len = true;
        } else if arg.starts_with('-') && arg.len() > 1 {
            for b in arg.bytes().skip(1) {
                match b {
                    b'c' => opts.bytes = true,
                    b'm' => opts.chars = true,
                    b'l' => opts.lines = true,
                    b'w' => opts.words = true,
                    b'L' => opts.max_line_len = true,
                    b'h' => {
                        print_help();
                        exit(0);
                    }
                    _ => {}
                }
            }
        } else if file_count < files.len() {
            files[file_count] = arg;
            file_count += 1;
        }
    }

    if !opts.any_set() {
        opts.lines = true;
        opts.words = true;
        opts.bytes = true;
    }

    if file_count == 0 {
        let c = count_fd(STDIN_FILENO);
        print_counts(c, opts, None);
        exit(0);
    }

    let mut total = Counts::default();
    let mut exit_code = 0u64;

    for &path in &files[..file_count] {
        if path == "-" {
            let c = count_fd(STDIN_FILENO);
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

        let mut null_buf = [0u8; 512];
        let b = path.as_bytes();
        if b.len() >= null_buf.len() {
            exit_code = 1;
            continue;
        }
        null_buf[..b.len()].copy_from_slice(b);
        null_buf[b.len()] = 0;

        let fd = open(&null_buf[..b.len() + 1], O_RDONLY as u64, 0);
        if fd < 0 {
            write(STDERR_FILENO, b"wc: ");
            write(STDERR_FILENO, b);
            write(STDERR_FILENO, b": No such file or directory\n");
            exit_code = 1;
            continue;
        }
        let c = count_fd(fd as u64);
        let _ = close(fd as u64);
        print_counts(c, opts, Some(path));
        total.lines += c.lines;
        total.words += c.words;
        total.bytes += c.bytes;
        total.chars += c.chars;
        if c.max_line_len > total.max_line_len {
            total.max_line_len = c.max_line_len;
        }
    }

    if file_count > 1 {
        print_counts(total, opts, Some("total"));
    }
    exit(exit_code);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    write(STDERR_FILENO, b"panic in wc\n");
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_state_counts_lines_words_bytes_chars() {
        let mut st = CounterState::new();
        st.feed("Привет мир\nhello\n".as_bytes());
        let c = st.finish();
        assert_eq!(c.lines, 2);
        assert_eq!(c.words, 3);
        assert_eq!(c.bytes, "Привет мир\nhello\n".len() as u64);
        assert_eq!(c.chars, 17);
        assert_eq!(c.max_line_len, 10);
    }
}
