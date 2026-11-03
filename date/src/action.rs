use libwunix::time::{clock_realtime, clock_set_realtime, get_system_timezone, DateTime, Timespec};
use libwunix::{println, t, Stat};
use crate::args::{DateArgs, IsoFormat};

pub fn run(args: DateArgs) -> i32 {
    let (tz_offset, tz_name, _) = if args.is_utc {
        (0i32, "UTC", "UTC")
    } else {
        get_system_timezone()
    };

    let now_ts = clock_realtime();
    let now_dt = DateTime::from_epoch(now_ts.tv_sec, now_ts.tv_nsec as u32, tz_offset);

    // 1. Установка времени по флагу -s / --set
    if let Some(target) = args.set_str {
        return match DateTime::parse(target, &now_dt) {
            Some(dt) => {
                let ep = dt.to_epoch(tz_offset);
                let ts = Timespec {
                    tv_sec: ep,
                    tv_nsec: 0,
                };
                if clock_set_realtime(&ts) == 0 {
                    print_datetime(&dt, tz_name, tz_offset, &args);
                    0
                } else {
                    println!(
                        "{}",
                        t(
                            "date: невозможно установить системное время: операция требует прав администратора",
                            "date: cannot set system date: operation not permitted"
                        )
                    );
                    1
                }
            }
            None => {
                println!(
                    "{}: '{}'",
                    t("date: неверный формат даты для установки", "date: invalid date specification"),
                    target
                );
                1
            }
        };
    }

    // 2. Получение целевой даты (из файла по -r, из строки по -d или текущее системное время)
    let target_dt = if let Some(ref_file) = args.reference_file {
        let mut st = Stat::default();
        let mut path_buf = [0u8; 256];
        let bytes = ref_file.as_bytes();
        if bytes.len() >= path_buf.len() {
            println!("{}", t("date: путь к файлу слишком длинный", "date: path too long"));
            return 1;
        }
        path_buf[..bytes.len()].copy_from_slice(bytes);
        path_buf[bytes.len()] = 0;

        if libwunix::stat(&path_buf[..bytes.len() + 1], &mut st) < 0 {
            println!(
                "{}: '{}'",
                t("date: невозможно прочитать статус файла", "date: cannot stat reference file"),
                ref_file
            );
            return 1;
        }
        DateTime::from_epoch(st.st_mtime, st.st_mtime_nsec as u32, tz_offset)
    } else if let Some(custom) = args.date_str {
        match DateTime::parse(custom, &now_dt) {
            Some(dt) => dt,
            None => {
                println!(
                    "{}: '{}'",
                    t("date: неверный формат даты", "date: invalid date"),
                    custom
                );
                return 1;
            }
        }
    } else {
        now_dt
    };

    print_datetime(&target_dt, tz_name, tz_offset, &args);
    0
}

fn print_datetime(dt: &DateTime, tz_name: &str, tz_offset: i32, args: &DateArgs) {
    let mut buf = [0u8; 256];
    let is_ru = libwunix::current_language() == libwunix::Language::Russian;

    if let Some(ref fmt) = args.custom_format {
        let s = dt.format_strftime(fmt, tz_name, tz_offset, is_ru, &mut buf);
        println!("{}", s);
    } else if args.rfc_email {
        let s = dt.format_rfc5322(tz_offset, &mut buf);
        println!("{}", s);
    } else if let Some(ref iso) = args.iso_8601 {
        let s = match iso {
            IsoFormat::Date => dt.format_iso8601(false, tz_offset, &mut buf),
            _ => dt.format_iso8601(true, tz_offset, &mut buf),
        };
        println!("{}", s);
    } else {
        let s = dt.format_default(tz_name, is_ru, &mut buf);
        println!("{}", s);
    }
}
