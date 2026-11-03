use libwunix::net::{query_ntp_host, sync_system_time, NtpResult};
use libwunix::time::{clock_realtime, clock_set_realtime, get_system_timezone, DateTime, Timespec};
use libwunix::{close, open, println, t, unlink, write, O_CREAT, O_TRUNC, O_WRONLY};
use crate::timezones::find_timezone;

pub fn set_time(time_spec: &str) -> i32 {
    let (tz_offset, _, _) = get_system_timezone();
    let now = clock_realtime();
    let current = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, tz_offset);

    match DateTime::parse(time_spec, &current) {
        Some(dt) => {
            let ep = dt.to_epoch(tz_offset);
            let ts = Timespec {
                tv_sec: ep,
                tv_nsec: 0,
            };
            if clock_set_realtime(&ts) == 0 {
                println!(
                    "{}",
                    t(
                        "timedatectl: системное время успешно установлено.",
                        "timedatectl: system time successfully updated."
                    )
                );
                0
            } else {
                println!(
                    "{}",
                    t(
                        "timedatectl: ошибка установки времени: требуются права суперпользователя (root).",
                        "timedatectl: failed to set time: operation not permitted (root required)."
                    )
                );
                1
            }
        }
        None => {
            println!(
                "{}: '{}'",
                t("timedatectl: неверный формат даты/времени", "timedatectl: invalid date/time specification"),
                time_spec
            );
            1
        }
    }
}

pub fn set_timezone(tz_name: &str) -> i32 {
    if find_timezone(tz_name).is_none() {
        println!(
            "{}: '{}'. {}",
            t("timedatectl: неизвестный часовой пояс", "timedatectl: unknown timezone"),
            tz_name,
            t("Используйте 'timedatectl list-timezones'.", "Use 'timedatectl list-timezones'.")
        );
        return 1;
    }

    let fd = open(b"/etc/timezone\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd < 0 {
        println!(
            "{}",
            t(
                "timedatectl: ошибка записи /etc/timezone: требуются права администратора.",
                "timedatectl: failed to write /etc/timezone: permission denied."
            )
        );
        return 1;
    }

    write(fd as u64, tz_name.as_bytes());
    write(fd as u64, b"\n");
    close(fd as u64);

    println!(
        "{}: {}",
        t("timedatectl: часовой пояс успешно изменен на", "timedatectl: timezone set to"),
        tz_name
    );
    0
}

pub fn set_local_rtc(enabled: bool) -> i32 {
    if enabled {
        println!(
            "{}",
            t(
                "Предупреждение: RTC в местном часовом поясе не рекомендуется для Unix-систем.",
                "Warning: RTC in local timezone is not recommended for Unix systems."
            )
        );
    }
    println!(
        "{}: {}",
        t("timedatectl: режим RTC в местном поясе", "timedatectl: RTC in local timezone"),
        if enabled { t("включен", "enabled") } else { t("выключен", "disabled") }
    );
    0
}

pub fn set_ntp(enabled: bool) -> i32 {
    let _ = libwunix::mkdir(b"/etc\0", 0o755);
    let fd = open(b"/etc/timesyncd.conf\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let conf = if enabled {
            "[Time]\nNTP=yes\nFallbackNTP=10.0.2.2\n"
        } else {
            "[Time]\nNTP=no\n"
        };
        write(fd as u64, conf.as_bytes());
        close(fd as u64);
    } else {
        println!(
            "{}",
            t(
                "timedatectl: ошибка записи /etc/timesyncd.conf: требуются права администратора.",
                "timedatectl: failed to write /etc/timesyncd.conf: permission denied."
            )
        );
        return 1;
    }

    if enabled {
        let servers = ["pool.ntp.org", "time.google.com", "10.0.2.2"];
        let mut synced = false;
        for &srv in &servers {
            if let Ok(res) = query_ntp_host(srv) {
                if sync_system_time(&res).is_ok() {
                    save_timesync_state(srv, &res);
                    synced = true;
                    break;
                }
            }
        }
        if synced {
            println!(
                "{}",
                t(
                    "timedatectl: сетевая синхронизация времени NTP активирована и системные часы обновлены.",
                    "timedatectl: NTP time synchronization enabled and system clock updated."
                )
            );
        } else {
            println!(
                "{}: {}",
                t("timedatectl: сетевая синхронизация времени NTP", "timedatectl: network time synchronization"),
                t("активирована (ожидание подключения к сети)", "active (waiting for network connectivity)")
            );
        }
    } else {
        let _ = unlink(b"/run/timesync/status\0");
        println!(
            "{}: {}",
            t("timedatectl: сетевая синхронизация времени NTP", "timedatectl: network time synchronization"),
            t("отключена", "inactive")
        );
    }

    0
}

fn save_timesync_state(server: &str, res: &NtpResult) {
    let _ = libwunix::mkdir(b"/run\0", 0o755);
    let _ = libwunix::mkdir(b"/run/timesync\0", 0o755);

    let fd = open(b"/run/timesync/status\0", (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let ip = res.server_ip;
        let mut buf = [0u8; 512];
        let mut cur = 0;

        let mut append = |s: &str| {
            let b = s.as_bytes();
            let n = b.len().min(buf.len() - cur);
            buf[cur..cur + n].copy_from_slice(&b[..n]);
            cur += n;
        };

        append("Server="); append(server); append("\n");
        append("ServerIP=");
        let mut ip_buf = [0u8; 20];
        append(format_ip(ip, &mut ip_buf)); append("\n");

        append("Stratum=");
        let mut num_buf = [0u8; 16];
        append(format_u32(res.stratum as u32, &mut num_buf)); append("\n");

        append("DelayMS=");
        append(format_u32(res.delay_ms as u32, &mut num_buf)); append("\n");

        append("OffsetSec=");
        if res.offset_sec < 0 || res.offset_nsec < 0 {
            append("-");
        } else {
            append("+");
        }
        append(format_u32(res.offset_sec.abs() as u32, &mut num_buf));
        append(".");
        let mut frac_buf = [0u8; 16];
        append(format_pad6((res.offset_nsec.abs() / 1000) as u32, &mut frac_buf));
        append("\n");

        append("PollIntervalSec=64\n");
        append("PacketCount=1\n");
        append("Synchronized=yes\n");

        write(fd as u64, &buf[..cur]);
        close(fd as u64);
    }
}

fn format_ip<'a>(ip: [u8; 4], buf: &'a mut [u8; 20]) -> &'a str {
    let mut idx = 0;
    for (i, &octet) in ip.iter().enumerate() {
        if i > 0 {
            buf[idx] = b'.';
            idx += 1;
        }
        let mut temp = [0u8; 3];
        let mut n = octet as usize;
        let mut digits = 0;
        if n == 0 {
            temp[0] = b'0';
            digits = 1;
        } else {
            while n > 0 {
                temp[digits] = b'0' + (n % 10) as u8;
                n /= 10;
                digits += 1;
            }
        }
        for d in 0..digits {
            buf[idx] = temp[digits - 1 - d];
            idx += 1;
        }
    }
    core::str::from_utf8(&buf[..idx]).unwrap_or("")
}

fn format_u32<'a>(mut n: u32, buf: &'a mut [u8; 16]) -> &'a str {
    if n == 0 {
        return "0";
    }
    let mut idx = 0;
    let mut temp = [0u8; 16];
    while n > 0 {
        temp[idx] = b'0' + (n % 10) as u8;
        n /= 10;
        idx += 1;
    }
    for i in 0..idx {
        buf[i] = temp[idx - 1 - i];
    }
    core::str::from_utf8(&buf[..idx]).unwrap_or("")
}

fn format_pad6<'a>(mut n: u32, buf: &'a mut [u8; 16]) -> &'a str {
    for i in (0..6).rev() {
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    core::str::from_utf8(&buf[..6]).unwrap_or("000000")
}
