use libwunix::net::{query_ntp_host, sync_system_time, NtpResult};
use libwunix::time::sleep_sec;
use libwunix::{close, open, println, read, t, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
use crate::args::TimesyncdArgs;

const DEFAULT_SERVERS: [&str; 4] = [
    "10.0.2.2",
    "pool.ntp.org",
    "time.google.com",
    "time.cloudflare.com",
];

pub fn run(args: TimesyncdArgs) -> i32 {
    let mut config_buf = [0u8; 1024];
    let (enabled, configured_servers, server_count) = load_config(&mut config_buf);

    if !enabled {
        if args.verbose {
            println!("{}", t("timesyncd: служба NTP отключена в /etc/timesyncd.conf", "timesyncd: NTP disabled in /etc/timesyncd.conf"));
        }
        return 0;
    }

    let servers: &[&str] = if server_count > 0 {
        &configured_servers[..server_count]
    } else {
        &DEFAULT_SERVERS
    };

    if args.verbose {
        println!("{}", t("timesyncd: запуск службы синхронизации времени Wunix", "timesyncd: starting Wunix time synchronization service"));
    }

    let mut packet_count = 0u32;
    let mut poll_interval = 64u32;

    loop {
        let mut synced = false;
        let max_tries = if args.once { 1 } else { servers.len() };
        for &server in &servers[..max_tries.min(servers.len())] {
            if args.verbose {
                println!("{}: {}", t("timesyncd: опрос сервера", "timesyncd: querying server"), server);
            }

            match query_ntp_host(server) {
                Ok(res) => {
                    packet_count = packet_count.saturating_add(1);
                    if let Err(e) = sync_system_time(&res) {
                        if args.verbose {
                            println!("{}: {}", t("timesyncd: ошибка применения времени", "timesyncd: failed to apply time"), e);
                        }
                    } else {
                        save_timesync_state(server, &res, packet_count, poll_interval);
                        synced = true;
                        if args.verbose {
                            println!(
                                "{}: {}, delay: {}ms",
                                t("timesyncd: системные часы успешно синхронизированы с сервером", "timesyncd: synchronized with server"),
                                server, res.delay_ms
                            );
                        }
                        break;
                    }
                }
                Err(_e) => {
                    if args.verbose {
                        println!("{}: {}", t("timesyncd: сервер недоступен", "timesyncd: server unreachable"), server);
                    }
                }
            }
        }

        if args.once {
            return 0;
        }

        // Адаптивный интервал опроса: при успехе держим 64..1024 сек, при сбое уменьшаем до 32 сек
        if synced {
            poll_interval = 64;
        } else {
            poll_interval = 32;
        }

        sleep_sec(poll_interval as u64);
    }
}

fn load_config<'a>(buf: &'a mut [u8; 1024]) -> (bool, [&'a str; 8], usize) {
    let mut servers: [&'a str; 8] = [""; 8];
    let mut count = 0;
    let mut enabled = true;

    let fd = open(b"/etc/timesyncd.conf\0", O_RDONLY as u64, 0);
    if fd >= 0 {
        let n = read(fd as u64, buf);
        close(fd as u64);
        if n > 0 {
            if let Ok(text) = core::str::from_utf8(&buf[..n as usize]) {
                for line in text.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("NTP=") {
                        let val = trimmed[4..].trim();
                        if val.eq_ignore_ascii_case("no")
                            || val.eq_ignore_ascii_case("false")
                            || val == "0"
                            || val.eq_ignore_ascii_case("off")
                        {
                            enabled = false;
                        } else {
                            for part in val.split_ascii_whitespace() {
                                if !part.is_empty() && count < servers.len() {
                                    servers[count] = part;
                                    count += 1;
                                }
                            }
                        }
                    } else if trimmed.starts_with("FallbackNTP=") {
                        let val = trimmed[12..].trim();
                        for part in val.split_ascii_whitespace() {
                            if !part.is_empty() && count < servers.len() {
                                servers[count] = part;
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    (enabled, servers, count)
}

fn save_timesync_state(server: &str, res: &NtpResult, packet_count: u32, poll_interval: u32) {
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

        append("PollIntervalSec=");
        append(format_u32(poll_interval, &mut num_buf)); append("\n");

        append("PacketCount=");
        append(format_u32(packet_count, &mut num_buf)); append("\n");

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
