#![no_std]
#![no_main]

mod format;
mod hw;

use format::{render_json, render_short, render_tree};
use hw::{collect_inventory, HwBuffers};
use libwunix::{exit, println, t};

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
    let mut short_mode = false;
    let mut json_mode = false;
    let mut filter_class: Option<&'static [u8]> = None;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: lshw [ОПЦИИ]...\n\
                     Динамическая инвентаризация аппаратного обеспечения компьютера.\n\n\
                     Опции:\n  \
                       -short                  Вывести компактную таблицу аппаратных компонентов\n  \
                       -class <класс>          Фильтровать вывод по классу оборудования (cpu, memory, disk, network, display, bus)\n  \
                       -json                   Вывести конфигурацию в формате JSON\n  \
                       -h, --help              Показать эту справку и выйти\n  \
                       -V, --version           Вывести информацию о версии и выйти",
                    "Usage: lshw [OPTIONS]...\n\
                     Dynamic hardware inventory lister.\n\n\
                     Options:\n  \
                       -short                  Output hardware tree in a compact table\n  \
                       -class <class>          Filter by hardware class (cpu, memory, disk, network, display, bus)\n  \
                       -json                   Dump hardware tree as JSON\n  \
                       -h, --help              Display this help and exit\n  \
                       -V, --version           Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("lshw (Wunix hardware-lister) 2.0.0");
            exit(0);
        } else if arg == b"-short" {
            short_mode = true;
        } else if arg == b"-json" {
            json_mode = true;
        } else if arg == b"-class" && i + 1 < argc {
            i += 1;
            filter_class = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
        }
        i += 1;
    }

    let mut bufs = HwBuffers::new();
    let inv = collect_inventory(&mut bufs);

    if json_mode {
        render_json(&inv, filter_class);
    } else if short_mode {
        render_short(&inv, filter_class);
    } else {
        render_tree(&inv, filter_class);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
