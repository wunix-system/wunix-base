#![no_std]
#![no_main]

mod addr;
mod iface;
mod route;
mod util;

use libwunix::{exit, println, t};

use addr::{show_addr, show_link};
use iface::{load_network_interfaces, NetIface};
use route::show_route;
use util::cstr_to_slice;

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut interfaces = [NetIface::default(); 8];
    let if_count = load_network_interfaces(&mut interfaces);

    // Обработка параметров утилиты ip (iproute2)
    let mut obj = b"addr" as &[u8];
    let mut brief = false;
    let mut idx = 1;

    while idx < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(idx as usize) });
        if arg == b"-br" || arg == b"-brief" {
            brief = true;
        } else if arg == b"-V" || arg == b"-Version" || arg == b"--version" {
            println!("ip utility, Wunix iproute2 1.0 (x86_64)");
            exit(0);
        } else if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: ip [ОПЦИИ] ОБЪЕКТ [КОМАНДА]\n\nОбъекты:\n  link, l     - сетевые устройства\n  addr, a     - адреса протоколов (IPv4/IPv6)\n  route, r    - таблица маршрутизации\n\nОпции:\n  -br, -brief - компактный вывод в одну строку\n  -V, -Version- показать версию",
                    "Usage: ip [OPTIONS] OBJECT [COMMAND]\n\nObjects:\n  link, l     - Network devices\n  addr, a     - Protocol addresses (IPv4/IPv6)\n  route, r    - Routing table\n\nOptions:\n  -br, -brief - Compact one-line output\n  -V, -Version- Display version information"
                )
            );
            exit(0);
        } else if !arg.starts_with(b"-") {
            obj = arg;
            break;
        }
        idx += 1;
    }

    if obj == b"addr" || obj == b"address" || obj == b"a" {
        show_addr(brief, &interfaces, if_count);
    } else if obj == b"link" || obj == b"l" {
        show_link(brief, &interfaces, if_count);
    } else if obj == b"route" || obj == b"r" || obj == b"ro" {
        show_route(&interfaces, if_count);
    } else {
        println!(
            "{}: '{}'",
            t("Неизвестный объект", "Object not found"),
            core::str::from_utf8(obj).unwrap_or("?")
        );
        println!(
            "{}",
            t(
                "Допустимые объекты: 'address' (addr, a), 'link' (l), 'route' (r).",
                "Supported objects: 'address' (addr, a), 'link' (l), 'route' (r)."
            )
        );
        exit(1);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    libwunix::write(libwunix::STDERR_FILENO, b"panic in ip\n");
    exit(1);
}
