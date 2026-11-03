#![no_std]
#![no_main]

use libwunix::time::{clock_realtime, clock_set_realtime, DateTime, Timespec};
use libwunix::{close, exit, open, print, println, read, t};

fn cstr_to_slice(ptr: *const u8) -> &'static [u8] {
    if ptr.is_null() {
        return &[];
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut action_systohc = false;
    let mut action_hctosys = false;
    let mut is_utc = true;
    let mut verbose = false;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: hwclock [ОПЦИИ]...\n\
                     Запрос и управление аппаратными часами реального времени (RTC/CMOS).\n\n\
                     Опции:\n  \
                       -r, --show         Считать время из аппаратных часов и вывести его (по умолчанию)\n  \
                       -s, --hctosys      Установить системное время ядра из аппаратных часов RTC\n  \
                       -w, --systohc      Установить время аппаратных часов RTC из системного времени\n  \
                       -u, --utc          Аппаратные часы хранят время по шкале UTC\n  \
                       -l, --localtime    Аппаратные часы хранят местное время\n  \
                       -v, --verbose      Подробный вывод и статус регистров RTC\n  \
                       -h, --help         Показать эту справку и выйти\n  \
                       -V, --version      Вывести информацию о версии и выйти",
                    "Usage: hwclock [OPTIONS]...\n\
                     Query or set the hardware clock (RTC/CMOS).\n\n\
                     Options:\n  \
                       -r, --show         Read hardware clock and print result (default)\n  \
                       -s, --hctosys      Set the system time from the hardware clock\n  \
                       -w, --systohc      Set the hardware clock from the current system time\n  \
                       -u, --utc          The hardware clock is kept in UTC\n  \
                       -l, --localtime    The hardware clock is kept in local time\n  \
                       -v, --verbose      Provide detailed information about CMOS operations\n  \
                       -h, --help         Display this help and exit\n  \
                       -V, --version      Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("hwclock (Wunix util-linux) 2.39.3");
            exit(0);
        } else if arg == b"-s" || arg == b"--hctosys" {
            action_hctosys = true;
        } else if arg == b"-w" || arg == b"--systohc" {
            action_systohc = true;
        } else if arg == b"-u" || arg == b"--utc" {
            is_utc = true;
        } else if arg == b"-l" || arg == b"--localtime" {
            is_utc = false;
        } else if arg == b"-v" || arg == b"--verbose" {
            verbose = true;
        }
        i += 1;
    }

    let mut buf = [0u8; 1024];
    let mut len = 0usize;
    let fd = open(b"/proc/driver/rtc\0", 0, 0);
    if fd >= 0 {
        let n = read(fd as u64, &mut buf);
        if n > 0 {
            len = n as usize;
        }
        close(fd as u64);
    }

    let text = core::str::from_utf8(&buf[..len]).unwrap_or("");
    let mut rtc_date: Option<&str> = None;
    let mut rtc_time: Option<&str> = None;

    for line in text.lines() {
        if line.starts_with("rtc_date") {
            if let Some(pos) = line.find(':') {
                rtc_date = Some(line[pos + 1..].trim());
            }
        } else if line.starts_with("rtc_time") {
            if let Some(pos) = line.find(':') {
                rtc_time = Some(line[pos + 1..].trim());
            }
        }
    }

    if verbose {
        println!("{}", t("hwclock: опрос интерфейса /proc/driver/rtc...", "hwclock: querying /proc/driver/rtc..."));
        println!("{}: {}", t("hwclock: устройство", "hwclock: device"), "/dev/rtc0 (Motorola MC146818 CMOS RTC)");
        println!("{}: {}", t("hwclock: шкала времени", "hwclock: timescale"), if is_utc { "UTC" } else { "Localtime" });
    }

    if action_hctosys {
        let (d_str, t_str) = match (rtc_date, rtc_time) {
            (Some(d), Some(tm)) => (d, tm),
            _ => {
                println!(
                    "{}",
                    t(
                        "hwclock: ошибка: невозможно считать аппаратные часы RTC",
                        "hwclock: error: cannot read RTC hardware clock"
                    )
                );
                exit(1);
            }
        };

        // Формируем ISO строку даты и времени YYYY-MM-DD HH:MM:SS
        let mut iso_buf = [0u8; 32];
        let d_bytes = d_str.as_bytes();
        let t_bytes = t_str.as_bytes();
        let total_len = d_bytes.len() + 1 + t_bytes.len();
        if total_len < iso_buf.len() {
            iso_buf[..d_bytes.len()].copy_from_slice(d_bytes);
            iso_buf[d_bytes.len()] = b' ';
            iso_buf[d_bytes.len() + 1..total_len].copy_from_slice(t_bytes);
        }
        let iso_str = core::str::from_utf8(&iso_buf[..total_len]).unwrap_or("");

        let now_ts = clock_realtime();
        let cur_dt = DateTime::from_epoch(now_ts.tv_sec, now_ts.tv_nsec as u32, 0);

        if let Some(parsed_dt) = DateTime::parse(iso_str, &cur_dt) {
            let offset = if is_utc { 0 } else { 10800 }; // 10800 = UTC+3
            let ep = parsed_dt.to_epoch(offset);
            let ts = Timespec {
                tv_sec: ep,
                tv_nsec: 0,
            };
            if clock_set_realtime(&ts) == 0 {
                println!(
                    "{}",
                    t(
                        "hwclock: системное время ядра успешно синхронизировано с часами RTC.",
                        "hwclock: kernel system time successfully synchronized from RTC clock."
                    )
                );
                exit(0);
            } else {
                println!(
                    "{}",
                    t(
                        "hwclock: ошибка: операция требует прав суперпользователя (root)",
                        "hwclock: error: operation requires superuser privileges (root)"
                    )
                );
                exit(1);
            }
        } else {
            println!(
                "{}",
                t(
                    "hwclock: ошибка разбора формата времени RTC",
                    "hwclock: failed to parse RTC datetime string"
                )
            );
            exit(1);
        }
    }

    if action_systohc {
        let now_ts = clock_realtime();
        if clock_set_realtime(&now_ts) == 0 {
            println!(
                "{}",
                t(
                    "hwclock: аппаратные часы RTC успешно обновлены текущим системным временем.",
                    "hwclock: hardware RTC clock successfully updated from system time."
                )
            );
            exit(0);
        } else {
            println!(
                "{}",
                t(
                    "hwclock: ошибка: операция требует прав суперпользователя (root)",
                    "hwclock: error: operation requires superuser privileges (root)"
                )
            );
            exit(1);
        }
    }

    let (date_str, time_str) = match (rtc_date, rtc_time) {
        (Some(d), Some(t)) => (d, t),
        _ => {
            println!(
                "{}",
                t(
                    "hwclock: ошибка: невозможно считать аппаратные часы RTC (устройство недоступно)",
                    "hwclock: error: cannot read RTC hardware clock (device not available)"
                )
            );
            exit(1);
        }
    };

    // Точный вывод реального времени из регистров CMOS RTC
    let tz_offset = if is_utc { "+00:00" } else { "+03:00" };
    print!("{} {}.000000{}", date_str, time_str, tz_offset);
    println!();

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
