#![no_std]
#![no_main]

mod format;

use libwunix::{
    exit, get_default_nameserver, parse_ipv4_bytes, println, resolve_host,
    resolve_host_with_server, t,
};

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
    let prog_name = if argc > 0 {
        cstr_to_slice(unsafe { *argv })
    } else {
        b"host"
    };

    let is_nslookup = prog_name.ends_with(b"nslookup");

    let mut target_domain: Option<&str> = None;
    let mut custom_server: Option<[u8; 4]> = None;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        let arg_s = core::str::from_utf8(arg).unwrap_or("");

        if arg == b"-h" || arg == b"--help" {
            print_help(is_nslookup);
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("host / nslookup (Wunix BIND) 1.0");
            exit(0);
        } else if target_domain.is_none() && !arg.starts_with(b"-") {
            target_domain = Some(arg_s);
        } else if custom_server.is_none() && !arg.starts_with(b"-") {
            if let Some(ip) = parse_ipv4_bytes(arg) {
                custom_server = Some(ip);
            }
        }
        i += 1;
    }

    let domain = match target_domain {
        Some(d) => d,
        None => {
            print_help(is_nslookup);
            exit(1);
        }
    };

    let server_ip = custom_server.unwrap_or_else(get_default_nameserver);

    let res = match custom_server {
        Some(srv) => resolve_host_with_server(domain, srv),
        None => resolve_host(domain),
    };

    match res {
        Ok(ip) => {
            if is_nslookup {
                format::print_nslookup_result(server_ip, domain, ip);
            } else {
                format::print_host_result(domain, ip);
            }
            exit(0);
        }
        Err(e) => {
            format::print_error(domain, e, is_nslookup);
            exit(1);
        }
    }
}

fn print_help(is_nslookup: bool) {
    if is_nslookup {
        println!(
            "{}",
            t(
                "Использование: nslookup <доменное_имя> [DNS-сервер]\n\
                 Интерактивный и пакетный запрос к серверам доменных имен.\n\n\
                 Примеры:\n  \
                   nslookup ya.ru\n  \
                   nslookup google.com 8.8.8.8",
                "Usage: nslookup <domain_name> [dns_server]\n\
                 Query Internet name servers interactively or non-interactively.\n\n\
                 Examples:\n  \
                   nslookup ya.ru\n  \
                   nslookup google.com 8.8.8.8"
            )
        );
    } else {
        println!(
            "{}",
            t(
                "Использование: host <доменное_имя> [DNS-сервер]\n\
                 Утилита разрешения доменных имен DNS.\n\n\
                 Примеры:\n  \
                   host ya.ru\n  \
                   host google.com 8.8.8.8",
                "Usage: host <domain_name> [dns_server]\n\
                 DNS lookup utility.\n\n\
                 Examples:\n  \
                   host ya.ru\n  \
                   host google.com 8.8.8.8"
            )
        );
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(libwunix::STDERR_FILENO, b"panic in host\n");
    exit(1);
}
