#![no_std]
#![no_main]

mod icmp;
mod stats;
mod time;

use icmp::{build_echo_packet, parse_u32};
use libwunix::{
    close, exit, println, recvfrom, sendto, socket, t, SockAddrIn, AF_INET, IPPROTO_ICMP,
    SOCK_RAW,
};
use stats::PingStats;
use time::{get_time_ms, sleep_ms};

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
    let mut target_ip_str: Option<&'static [u8]> = None;
    let mut count_opt: Option<u32> = None;
    let mut data_size: u32 = 56;
    let mut timeout_ms: u64 = 1000;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование:\n  \
                       ping [-c число] [-s размер] [-W таймаут] [-4] <хост>\n\n\
                     Опции:\n  \
                       -c число       Число отправляемых пакетов эхо-запроса.\n  \
                       -s размер      Размер данных полезной нагрузки в байтах (по умолчанию: 56).\n  \
                       -W таймаут     Время ожидания ответа в секундах (по умолчанию: 1).\n  \
                       -4             Принудительное использование протокола IPv4.\n  \
                       -h, --help     Показать эту справку и выйти.\n  \
                       -V, --version  Показать версию утилиты.",
                    "Usage:\n  \
                       ping [-c count] [-s size] [-W timeout] [-4] <target>\n\n\
                     Options:\n  \
                       -c count       Number of echo requests to send.\n  \
                       -s size        Number of data bytes to send (default: 56).\n  \
                       -W timeout     Time to wait for a response in seconds (default: 1).\n  \
                       -4             Force using IPv4.\n  \
                       -h, --help     Display this help and exit.\n  \
                       -V, --version  Display version."
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("ping (Wunix iputils) 1.0.0");
            exit(0);
        } else if arg == b"-c" {
            i += 1;
            if i < argc {
                let val_str = cstr_to_slice(unsafe { *argv.add(i as usize) });
                if let Some(c) = parse_u32(val_str) {
                    count_opt = Some(c);
                }
            }
        } else if arg.starts_with(b"-c") && arg.len() > 2 {
            if let Some(c) = parse_u32(&arg[2..]) {
                count_opt = Some(c);
            }
        } else if arg == b"-s" {
            i += 1;
            if i < argc {
                let val_str = cstr_to_slice(unsafe { *argv.add(i as usize) });
                if let Some(sz) = parse_u32(val_str) {
                    data_size = sz.clamp(0, 1400);
                }
            }
        } else if arg == b"-W" {
            i += 1;
            if i < argc {
                let val_str = cstr_to_slice(unsafe { *argv.add(i as usize) });
                if let Some(to) = parse_u32(val_str) {
                    timeout_ms = (to as u64) * 1000;
                }
            }
        } else if arg == b"-4" {
            // Флаг IPv4 принят
        } else if !arg.starts_with(b"-") {
            target_ip_str = Some(arg);
        }
        i += 1;
    }

    let infinite = count_opt.is_none();
    let count = count_opt.unwrap_or(u32::MAX);

    let target_str = match target_ip_str {
        Some(s) => s,
        None => {
            println!(
                "{}",
                t(
                    "ping: необходимо указать имя узла или IP-адрес назначения.",
                    "ping: host name or IP address must be specified."
                )
            );
            exit(1);
        }
    };

    let host_s = core::str::from_utf8(target_str).unwrap_or("");
    let target_ip = match libwunix::resolve_host(host_s) {
        Ok(ip) => ip,
        Err(_) => {
            println!(
                "{}: {}",
                t("ping: не удалось разрешить имя узла", "ping: cannot resolve host"),
                host_s
            );
            exit(1);
        }
    };

    let fd = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
    if fd < 0 {
        println!(
            "{}: {}",
            t("ping: ошибка открытия ICMP сокета", "ping: socket creation error"),
            fd
        );
        exit(1);
    }

    let dest_sockaddr = SockAddrIn::new_v4(target_ip, 0);
    let ip_s = core::str::from_utf8(target_str).unwrap_or("");

    let total_ip_len = data_size + 28;
    println!("PING {} ({}) {}({}) bytes of data.", ip_s, ip_s, data_size, total_ip_len);

    let mut stats = PingStats::new();
    let total_pkt_len = 8 + (data_size as usize);
    let mut packet = [0u8; 1500];
    let mut rx_buf = [0u8; 1500];
    let ident: u16 = 0x0001;

    let mut seq = 1u32;
    while infinite || seq <= count {
        build_echo_packet(&mut packet, ident, seq, data_size as usize);
        let pkt_slice = &packet[..total_pkt_len];

        let t_send = get_time_ms();
        let sent = sendto(fd as u64, pkt_slice, 0, Some(&dest_sockaddr));
        if sent >= 0 {
            stats.record_transmit();

            let mut got_reply = false;
            let wait_start = get_time_ms();
            while get_time_ms().saturating_sub(wait_start) < timeout_ms {
                let mut from_addr = SockAddrIn::default();
                let n = recvfrom(fd as u64, &mut rx_buf, 0, Some(&mut from_addr));
                if n >= 8 {
                    let rtt = get_time_ms().saturating_sub(t_send);
                    let icmp_type = rx_buf[0];
                    let r_ident = u16::from_be_bytes([rx_buf[4], rx_buf[5]]);
                    let r_seq = u16::from_be_bytes([rx_buf[6], rx_buf[7]]);

                    if icmp_type == 0 && (r_ident == ident || r_ident == 0) && r_seq as u32 == seq {
                        stats.record_reply(rtt);
                        got_reply = true;

                        let ttl = 64;
                        let reply_bytes = data_size + 8;
                        println!(
                            "{} bytes from {}: icmp_seq={} ttl={} time={} ms",
                            reply_bytes, ip_s, seq, ttl, rtt
                        );
                        break;
                    }
                }
                libwunix::yield_now();
            }

            if !got_reply {
                println!("Request timeout for icmp_seq {}", seq);
            }
        } else {
            println!(
                "{}",
                t("Сбой передачи. General failure.", "Transmit failed. General failure.")
            );
        }

        if infinite || seq < count {
            sleep_ms(1000);
        }
        seq += 1;
    }

    close(fd as u64);
    stats.print_summary(ip_s);

    let lost = stats.transmitted.saturating_sub(stats.received);
    exit(if lost == 0 { 0 } else { 1 });
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
