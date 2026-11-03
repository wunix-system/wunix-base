#![no_std]
#![no_main]

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

fn parse_hex_byte(s: &[u8]) -> Option<u8> {
    if s.len() < 2 {
        return None;
    }
    let hi = match s[0] {
        b'0'..=b'9' => s[0] - b'0',
        b'a'..=b'f' => s[0] - b'a' + 10,
        b'A'..=b'F' => s[0] - b'A' + 10,
        _ => return None,
    };
    let lo = match s[1] {
        b'0'..=b'9' => s[1] - b'0',
        b'a'..=b'f' => s[1] - b'a' + 10,
        b'A'..=b'F' => s[1] - b'A' + 10,
        _ => return None,
    };
    Some((hi << 4) | lo)
}

fn parse_hex_u16(s: &[u8]) -> Option<u16> {
    if s.len() < 4 {
        return None;
    }
    let b0 = parse_hex_byte(&s[0..2])?;
    let b1 = parse_hex_byte(&s[2..4])?;
    Some(((b0 as u16) << 8) | (b1 as u16))
}

fn print_ip_port(hex_ip: &[u8], hex_port: &[u8]) {
    if hex_ip.len() == 8 {
        let b0 = parse_hex_byte(&hex_ip[6..8]).unwrap_or(0);
        let b1 = parse_hex_byte(&hex_ip[4..6]).unwrap_or(0);
        let b2 = parse_hex_byte(&hex_ip[2..4]).unwrap_or(0);
        let b3 = parse_hex_byte(&hex_ip[0..2]).unwrap_or(0);
        let port = parse_hex_u16(hex_port).unwrap_or(0);

        if port == 0 {
            print!("{}.{}.{}.{}:*               ", b0, b1, b2, b3);
        } else {
            print!("{}.{}.{}.{}:{}           ", b0, b1, b2, b3, port);
        }
    } else {
        print!("0.0.0.0:*               ");
    }
}

fn state_str(st: u8) -> &'static str {
    match st {
        1 => "ESTAB",
        2 => "SYN-SENT",
        3 => "SYN-RECV",
        4 => "FIN-WAIT-1",
        5 => "FIN-WAIT-2",
        6 => "TIME-WAIT",
        7 => "UNCONN",
        8 => "CLOSE-WAIT",
        9 => "LAST-ACK",
        10 => "LISTEN",
        11 => "CLOSING",
        _ => "UNKNOWN",
    }
}

fn dump_proto(path: &[u8], proto_name: &str, only_listen: bool) {
    let fd = open(path, 0 /* O_RDONLY */, 0);
    if fd < 0 {
        return;
    }

    let mut buf = [0u8; 4096];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n <= 0 {
        return;
    }

    let content = match core::str::from_utf8(&buf[..n as usize]) {
        Ok(s) => s,
        Err(_) => return,
    };

    for (line_idx, line) in content.lines().enumerate() {
        if line_idx == 0 {
            continue; // Заголовок
        }
        let mut fields = line.split_whitespace();
        let _sl = match fields.next() {
            Some(f) => f,
            None => continue,
        };
        let local = match fields.next() {
            Some(f) => f,
            None => continue,
        };
        let remote = match fields.next() {
            Some(f) => f,
            None => continue,
        };
        let state_hex = match fields.next() {
            Some(f) => f,
            None => continue,
        };

        let state_val = parse_hex_byte(state_hex.as_bytes()).unwrap_or(0);
        let is_listen = state_val == 10;

        if only_listen && !is_listen {
            continue;
        }

        print!("{:<7} {:<10} {:<7} {:<7} ", proto_name, state_str(state_val), 0, 0);

        let mut local_parts = local.split(':');
        if let (Some(lip), Some(lport)) = (local_parts.next(), local_parts.next()) {
            print_ip_port(lip.as_bytes(), lport.as_bytes());
        }

        let mut remote_parts = remote.split(':');
        if let (Some(rip), Some(rport)) = (remote_parts.next(), remote_parts.next()) {
            print_ip_port(rip.as_bytes(), rport.as_bytes());
        }

        println!();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut show_tcp = false;
    let mut show_udp = false;
    let mut only_listening = false;

    for i in 1..argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: ss [ОПЦИИ]\n\
                     Опции:\n  \
                       -t, --tcp         Показывать сокеты TCP\n  \
                       -u, --udp         Показывать сокеты UDP\n  \
                       -l, --listening   Показывать только слушающие сокеты\n  \
                       -a, --all         Показывать все сокеты (слушающие и установленные)\n  \
                       -n, --numeric     Не разрешать имена портов\n  \
                       -h, --help        Справка по использованию\n  \
                       -V, --version     Версия утилиты",
                    "Usage: ss [OPTIONS]\n\
                     Options:\n  \
                       -t, --tcp         Display TCP sockets\n  \
                       -u, --udp         Display UDP sockets\n  \
                       -l, --listening   Display only listening sockets\n  \
                       -a, --all         Display both listening and non-listening sockets\n  \
                       -n, --numeric     Do not resolve service names\n  \
                       -h, --help        Display this help and exit\n  \
                       -V, --version     Display version"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("ss (Wunix iproute2) 1.0.0");
            exit(0);
        } else if arg == b"-t" || arg == b"--tcp" {
            show_tcp = true;
        } else if arg == b"-u" || arg == b"--udp" {
            show_udp = true;
        } else if arg == b"-l" || arg == b"--listening" {
            only_listening = true;
        } else if arg == b"-a" || arg == b"--all" {
            // По умолчанию
        }
    }

    if !show_tcp && !show_udp {
        show_tcp = true;
        show_udp = true;
    }

    println!(
        "{}",
        t(
            "Сетевой Статус     Recv-Q  Send-Q  Локальный адрес:Порт     Удаленный адрес:Порт",
            "Netid   State      Recv-Q  Send-Q  Local Address:Port       Peer Address:Port"
        )
    );

    if show_tcp {
        dump_proto(b"/proc/net/tcp\0", "tcp", only_listening);
    }
    if show_udp {
        dump_proto(b"/proc/net/udp\0", "udp", only_listening);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("ss: panic occurred");
    exit(1);
}
