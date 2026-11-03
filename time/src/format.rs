use libwunix::fmt::{itoa, utoa};
use libwunix::{close, open, write, O_CREAT, O_TRUNC, O_WRONLY, STDERR_FILENO};
use crate::args::TimeArgs;

// Флаг добавления в конец файла (O_APPEND в POSIX)
const O_APPEND: u32 = 0x400;

pub struct TimeResult<'a> {
    pub elapsed_ms: u64,
    pub exit_status: i32,
    pub cmd_name: &'a str,
}

pub fn output_results(res: &TimeResult, args: &TimeArgs) {
    let mut buf = [0u8; 1024];
    let len = format_report(res, args, &mut buf);

    if let Some(path) = args.output_file {
        let mut null_path = [0u8; 256];
        let bytes = path.as_bytes();
        if bytes.len() < null_path.len() {
            null_path[..bytes.len()].copy_from_slice(bytes);
            null_path[bytes.len()] = 0;
            let flags = (O_WRONLY | O_CREAT | if args.append { O_APPEND } else { O_TRUNC }) as u64;
            let fd = open(&null_path[..bytes.len() + 1], flags, 0o644);
            if fd >= 0 {
                write(fd as u64, &buf[..len]);
                close(fd as u64);
                return;
            }
        }
    }

    write(STDERR_FILENO, &buf[..len]);
}

fn format_report(res: &TimeResult, args: &TimeArgs, buf: &mut [u8]) -> usize {
    let mut idx = 0;

    let sec = res.elapsed_ms / 1000;
    let centis = (res.elapsed_ms % 1000) / 10;

    if let Some(fmt) = args.format {
        format_custom(res, fmt, buf, &mut idx);
    } else if args.portability {
        // POSIX standard: real %d.%02d\nuser 0.00\nsys 0.00\n
        append_str("real ", buf, &mut idx);
        append_u64(sec, buf, &mut idx);
        append_str(".", buf, &mut idx);
        append_2digits(centis as u32, buf, &mut idx);
        append_str("\nuser 0.00\nsys 0.00\n", buf, &mut idx);
    } else if args.verbose {
        append_str("\tCommand being timed: \"", buf, &mut idx);
        append_str(res.cmd_name, buf, &mut idx);
        append_str("\"\n\tUser time (seconds): 0.00\n\tSystem time (seconds): 0.00\n\tPercent of CPU this job got: 100%\n\tElapsed (wall clock) time (h:mm:ss or m:ss): ", buf, &mut idx);
        append_wall_time(res.elapsed_ms, buf, &mut idx);
        append_str("\n\tMajor (requiring I/O) page faults: 0\n\tMinor (reclaiming a frame) page faults: 0\n\tVoluntary context switches: 1\n\tInvoluntary context switches: 0\n\tExit status: ", buf, &mut idx);
        append_i64(res.exit_status as i64, buf, &mut idx);
        append_str("\n", buf, &mut idx);
    } else {
        // Стандартный краткий вывод GNU/Wunix
        append_str("\nreal\t", buf, &mut idx);
        append_wall_time(res.elapsed_ms, buf, &mut idx);
        append_str("\nuser\t0m0.000s\nsys\t0m0.000s\n", buf, &mut idx);
    }

    idx
}

fn format_custom(res: &TimeResult, fmt: &str, buf: &mut [u8], idx: &mut usize) {
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            i += 1;
            match bytes[i] {
                b'E' => append_wall_time(res.elapsed_ms, buf, idx),
                b'e' => {
                    append_u64(res.elapsed_ms / 1000, buf, idx);
                    append_str(".", buf, idx);
                    append_2digits(((res.elapsed_ms % 1000) / 10) as u32, buf, idx);
                }
                b'x' => append_i64(res.exit_status as i64, buf, idx),
                b'C' => append_str(res.cmd_name, buf, idx),
                b'S' => append_str("0.00", buf, idx),
                b'U' => append_str("0.00", buf, idx),
                b'P' => append_str("100%", buf, idx),
                b'%' => append_str("%", buf, idx),
                b'n' => append_str("\n", buf, idx),
                b't' => append_str("\t", buf, idx),
                other => {
                    append_str("%", buf, idx);
                    if *idx < buf.len() {
                        buf[*idx] = other;
                        *idx += 1;
                    }
                }
            }
        } else if *idx < buf.len() {
            buf[*idx] = bytes[i];
            *idx += 1;
        }
        i += 1;
    }
    if !fmt.ends_with('\n') {
        append_str("\n", buf, idx);
    }
}

fn append_wall_time(ms: u64, buf: &mut [u8], idx: &mut usize) {
    let total_sec = ms / 1000;
    let millis = ms % 1000;
    let minutes = total_sec / 60;
    let seconds = total_sec % 60;

    append_u64(minutes, buf, idx);
    append_str("m", buf, idx);
    append_2digits(seconds as u32, buf, idx);
    append_str(".", buf, idx);
    let mut num_buf = [0u8; 8];
    let ms_str = utoa(millis, &mut num_buf);
    if millis < 10 {
        append_str("00", buf, idx);
    } else if millis < 100 {
        append_str("0", buf, idx);
    }
    append_str(ms_str, buf, idx);
    append_str("s", buf, idx);
}

fn append_str(s: &str, buf: &mut [u8], idx: &mut usize) {
    for &b in s.as_bytes() {
        if *idx < buf.len() {
            buf[*idx] = b;
            *idx += 1;
        }
    }
}

fn append_u64(n: u64, buf: &mut [u8], idx: &mut usize) {
    let mut num_buf = [0u8; 24];
    let s = utoa(n, &mut num_buf);
    append_str(s, buf, idx);
}

fn append_i64(n: i64, buf: &mut [u8], idx: &mut usize) {
    let mut num_buf = [0u8; 24];
    let s = itoa(n, &mut num_buf);
    append_str(s, buf, idx);
}

fn append_2digits(n: u32, buf: &mut [u8], idx: &mut usize) {
    let b = [(b'0' + ((n / 10) % 10) as u8), (b'0' + (n % 10) as u8)];
    for &byte in &b {
        if *idx < buf.len() {
            buf[*idx] = byte;
            *idx += 1;
        }
    }
}
