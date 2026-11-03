use libwunix::net::{query_ntp_host_port, sync_system_time, NtpResult};
use libwunix::time::{clock_realtime, get_system_timezone, DateTime};
use libwunix::{close, open, print, println, read, t, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
use crate::args::NtpdateArgs;

const DEFAULT_SERVERS: [&str; 4] = [
    "pool.ntp.org",
    "time.google.com",
    "time.cloudflare.com",
    "10.0.2.2",
];

pub fn run(args: NtpdateArgs) -> i32 {
    let mut config_servers: [&str; 4] = [""; 4];
    let mut config_buf = [0u8; 512];
    let mut config_count = 0;

    let target_servers: &[&str] = if args.server_count > 0 {
        &args.servers[..args.server_count]
    } else {
        // Читаем список серверов из /etc/ntp.conf если доступен
        let fd = open(b"/etc/ntp.conf\0", O_RDONLY as u64, 0);
        if fd >= 0 {
            let n = read(fd as u64, &mut config_buf);
            close(fd as u64);
            if n > 0 {
                if let Ok(text) = core::str::from_utf8(&config_buf[..n as usize]) {
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("server ") {
                            let srv = trimmed[7..].trim();
                            if !srv.is_empty() && config_count < config_servers.len() {
                                config_servers[config_count] = srv;
                                config_count += 1;
                            }
                        }
                    }
                }
            }
        }
        if config_count > 0 {
            &config_servers[..config_count]
        } else {
            &DEFAULT_SERVERS
        }
    };

    let mut successful_result: Option<(&str, NtpResult)> = None;

    for &server in target_servers {
        if args.verbose {
            print!("{}: {}... ", t("ntpdate: опрос сервера", "ntpdate: querying server"), server);
        }

        match query_ntp_host_port(server, args.port) {
            Ok(res) => {
                if args.verbose {
                    println!("{}", t("OK", "OK"));
                }
                successful_result = Some((server, res));
                break;
            }
            Err(e) => {
                if args.verbose {
                    println!("{}: {:?}", t("ОШИБКА", "ERROR"), e);
                }
            }
        }
    }

    let (server_name, res) = match successful_result {
        Some((s, r)) => (s, r),
        None => {
            println!(
                "{}",
                t(
                    "ntpdate: подходящий сервер для синхронизации времени не найден",
                    "ntpdate: no server suitable for synchronization found"
                )
            );
            return 1;
        }
    };

    // Вычисляем смещение в секундах с плавающей точкой для красивой печати
    let ip = res.server_ip;
    let off_sign = if res.offset_sec >= 0 && res.offset_nsec >= 0 { "+" } else { "-" };
    let abs_sec = res.offset_sec.abs();
    let abs_usec = (res.offset_nsec.abs() / 1000) as u32;

    if !args.query_only {
        if let Err(err_msg) = sync_system_time(&res) {
            println!("{}: {}", t("ntpdate: ошибка установки времени", "ntpdate: failed to set system time"), err_msg);
            return 1;
        }

        // Обновляем системный статус синхронизации для timedatectl
        save_timesync_state(server_name, &res);
    }

    if !args.silent {
        let (tz_offset, _, _) = get_system_timezone();
        let now = clock_realtime();
        let dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, tz_offset);
        let mut time_buf = [0u8; 64];
        let time_str = dt.format_strftime("%d %b %H:%M:%S", "", tz_offset, false, &mut time_buf);

        if args.query_only {
            println!(
                "server {}.{}.{}.{}, stratum {}, offset {}{}.{:06} sec, delay {} ms",
                ip[0], ip[1], ip[2], ip[3], res.stratum, off_sign, abs_sec, abs_usec, res.delay_ms
            );
        } else {
            println!(
                "{} ntpdate[{}]: step time server {}.{}.{}.{} offset {}{}.{:06} sec",
                time_str, 1, ip[0], ip[1], ip[2], ip[3], off_sign, abs_sec, abs_usec
            );
        }
    }

    0
}

/// Сохраняет файл состояния синхронизации в `/run/timesync/status`
pub fn save_timesync_state(server: &str, res: &NtpResult) {
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
        let ip_s = format_ip(ip, &mut ip_buf);
        append(ip_s); append("\n");

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
