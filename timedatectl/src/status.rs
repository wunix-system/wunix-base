use libwunix::net::{query_ntp_host, LeapIndicator};
use libwunix::time::{clock_realtime, get_system_timezone, DateTime};
use libwunix::{close, open, print, println, read, t, O_RDONLY};

#[derive(Debug, Clone, Copy, Default)]
pub struct TimesyncState<'a> {
    pub server: &'a str,
    pub server_ip: &'a str,
    pub stratum: u8,
    pub delay_ms: u32,
    pub offset_str: &'a str,
    pub poll_interval: u32,
    pub packet_count: u32,
    pub synchronized: bool,
}

pub fn is_ntp_enabled() -> bool {
    let mut buf = [0u8; 256];
    let fd = open(b"/etc/timesyncd.conf\0", O_RDONLY as u64, 0);
    if fd >= 0 {
        let n = read(fd as u64, &mut buf);
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
                            return false;
                        }
                    }
                }
            }
        }
    }
    true
}

pub fn read_timesync_status<'a>(buf: &'a mut [u8; 1024]) -> Option<TimesyncState<'a>> {
    let fd = open(b"/run/timesync/status\0", O_RDONLY as u64, 0);
    if fd < 0 {
        return None;
    }
    let n = read(fd as u64, buf);
    close(fd as u64);
    if n <= 0 {
        return None;
    }

    let text = core::str::from_utf8(&buf[..n as usize]).ok()?;
    let mut state = TimesyncState::default();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find('=') {
            let key = &trimmed[..pos];
            let val = &trimmed[pos + 1..];
            match key {
                "Server" => state.server = val,
                "ServerIP" => state.server_ip = val,
                "Stratum" => state.stratum = parse_u32(val) as u8,
                "DelayMS" => state.delay_ms = parse_u32(val),
                "OffsetSec" => state.offset_str = val,
                "PollIntervalSec" => state.poll_interval = parse_u32(val),
                "PacketCount" => state.packet_count = parse_u32(val),
                "Synchronized" => state.synchronized = val == "yes" || val == "1" || val == "true",
                _ => {}
            }
        }
    }

    if state.server.is_empty() {
        None
    } else {
        Some(state)
    }
}

pub fn show_status() {
    let (tz_offset, tz_abbr, tz_canonical) = get_system_timezone();
    let now = clock_realtime();
    let utc_dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, 0);
    let local_dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, tz_offset);

    let mut local_buf = [0u8; 128];
    let local_s = local_dt.format_strftime("%a %F %T %Z", tz_abbr, tz_offset, false, &mut local_buf);

    let mut utc_buf = [0u8; 128];
    let utc_s = utc_dt.format_strftime("%a %F %T UTC", "UTC", 0, false, &mut utc_buf);

    let mut rtc_buf = [0u8; 1024];
    let (rtc_date, rtc_time) = read_rtc_datetime(&mut rtc_buf);

    let mut rtc_combined = [0u8; 128];
    let rtc_str = match (rtc_date, rtc_time) {
        (Some(d), Some(tm)) => {
            let mut idx = 0;
            for &b in d.as_bytes().iter().chain(b" ".iter()).chain(tm.as_bytes().iter()) {
                if idx < rtc_combined.len() {
                    rtc_combined[idx] = b;
                    idx += 1;
                }
            }
            core::str::from_utf8(&rtc_combined[..idx]).unwrap_or("n/a")
        }
        _ => "n/a",
    };

    let sign = if tz_offset >= 0 { "+" } else { "-" };
    let abs_m = (tz_offset.abs() / 60) as u32;
    let zh = abs_m / 60;
    let zm = abs_m % 60;

    let mut sync_buf = [0u8; 1024];
    let timesync = read_timesync_status(&mut sync_buf);
    let is_synced = timesync.map(|s| s.synchronized).unwrap_or(false);
    let ntp_active = is_ntp_enabled();

    println!("{}: {}", t("               Местное время", "               Local time"), local_s);
    println!("{}: {}", t("             Всемирное время", "           Universal time"), utc_s);
    println!("{}: {}", t("             Время часов RTC", "                 RTC time"), rtc_str);
    print!("{}: {} ({}, {}{:02}{:02})", t("                Часовой пояс", "                Time zone"), tz_canonical, tz_abbr, sign, zh, zm);
    println!();
    println!(
        "{}: {}",
        t("  Системные часы синхронизированы", "System clock synchronized"),
        if is_synced { t("да", "yes") } else { t("нет", "no") }
    );
    println!(
        "{}: {}",
        t("                      Служба NTP", "              NTP service"),
        if ntp_active { t("активна", "active") } else { t("отключена", "inactive") }
    );
    println!("{}: {}", t("             RTC в местном поясе", "          RTC in local TZ"), t("нет", "no"));
}

pub fn show_properties() {
    let (tz_offset, tz_abbr, tz_canonical) = get_system_timezone();
    let now = clock_realtime();
    let utc_dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, 0);
    let local_dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, tz_offset);

    let mut local_buf = [0u8; 128];
    let local_s = local_dt.format_strftime("%a %F %T %Z", tz_abbr, tz_offset, false, &mut local_buf);

    let mut utc_buf = [0u8; 128];
    let utc_s = utc_dt.format_strftime("%a %F %T UTC", "UTC", 0, false, &mut utc_buf);

    let mut rtc_buf = [0u8; 1024];
    let (rtc_date, rtc_time) = read_rtc_datetime(&mut rtc_buf);

    let mut sync_buf = [0u8; 1024];
    let timesync = read_timesync_status(&mut sync_buf);
    let is_synced = timesync.map(|s| s.synchronized).unwrap_or(false);
    let ntp_active = is_ntp_enabled();

    println!("Timezone={}", tz_canonical);
    println!("LocalTime={}", local_s);
    println!("UniversalTime={}", utc_s);
    if let (Some(d), Some(tm)) = (rtc_date, rtc_time) {
        println!("RTCDate={}", d);
        println!("RTCTime={}", tm);
    }
    println!("LocalRTC=no");
    println!("CanNTP=yes");
    println!("NTP={}", if ntp_active { "yes" } else { "no" });
    println!("NTPSynchronized={}", if is_synced { "yes" } else { "no" });
    println!("TimeUSec={}", now.tv_sec as u64 * 1_000_000 + now.tv_nsec as u64 / 1_000);
}

pub fn show_timesync_status() {
    let mut sync_buf = [0u8; 1024];
    if let Some(state) = read_timesync_status(&mut sync_buf) {
        println!("{}", t("Служба синхронизации времени Wunix TimeSync:", "Wunix TimeSync Network Time Service:"));
        println!("       Server: {} ({})", state.server, state.server_ip);
        println!("     Poll interval: {}s", if state.poll_interval > 0 { state.poll_interval } else { 64 });
        println!("      Leap status: normal");
        println!("          Version: 4");
        println!("          Stratum: {}", state.stratum);
        println!("       Delay: {}ms", state.delay_ms);
        println!("      Offset: {}s", state.offset_str);
        println!("  Packet count: {}", if state.packet_count > 0 { state.packet_count } else { 1 });
        return;
    }

    // Если файл статуса еще не сохранен, пробуем выполнить прямой опрос NTP-сервера
    println!("{}", t("Опрос сервера синхронизации времени...", "Probing network time server..."));
    let probe_server = "pool.ntp.org";
    match query_ntp_host(probe_server) {
        Ok(res) => {
            let ip = res.server_ip;
            let off_sign = if res.offset_sec >= 0 && res.offset_nsec >= 0 { "+" } else { "-" };
            let abs_sec = res.offset_sec.abs();
            let abs_usec = (res.offset_nsec.abs() / 1000) as u32;

            println!("{}", t("Служба синхронизации времени Wunix TimeSync:", "Wunix TimeSync Network Time Service:"));
            println!("       Server: {} ({}.{}.{}.{})", probe_server, ip[0], ip[1], ip[2], ip[3]);
            println!("     Poll interval: 64s (min: 32s; max 2048s)");
            println!(
                "      Leap status: {}",
                match res.leap {
                    LeapIndicator::NoWarning => "normal",
                    LeapIndicator::LastMinute61Sec => "61sec",
                    LeapIndicator::LastMinute59Sec => "59sec",
                    LeapIndicator::Alarm => "unsynchronized",
                }
            );
            println!("          Version: {}", res.version);
            println!("          Stratum: {}", res.stratum);
            println!("       Delay: {}ms", res.delay_ms);
            println!("      Offset: {}{}.{:06}s", off_sign, abs_sec, abs_usec);
            println!("  Packet count: 1");
        }
        Err(_) => {
            println!(
                "{}",
                t(
                    "timedatectl: сетевая синхронизация времени еще не выполнялась (сервер pool.ntp.org)",
                    "timedatectl: network time sync not active yet (server pool.ntp.org)"
                )
            );
        }
    }
}

fn read_rtc_datetime(buf: &mut [u8; 1024]) -> (Option<&str>, Option<&str>) {
    let mut len = 0usize;
    let fd = open(b"/proc/driver/rtc\0", 0, 0);
    if fd >= 0 {
        let n = read(fd as u64, buf);
        if n > 0 {
            len = n as usize;
        }
        close(fd as u64);
    }

    let text = core::str::from_utf8(&buf[..len]).unwrap_or("");
    let mut rtc_date = None;
    let mut rtc_time = None;

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

    (rtc_date, rtc_time)
}

fn parse_u32(s: &str) -> u32 {
    let mut res = 0u32;
    for &b in s.as_bytes() {
        if b >= b'0' && b <= b'9' {
            res = res.saturating_mul(10).saturating_add((b - b'0') as u32);
        } else {
            break;
        }
    }
    res
}
