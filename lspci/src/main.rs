#![no_std]
#![no_main]

mod db;
mod pci;

use db::{pci_class_name, pci_device_desc, pci_vendor_name};
use libwunix::{exit, print, println, t};
use pci::{load_pci_devices, parse_hex, read_proc_pci_devices, PciDevEntry};

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

fn print_size(sz: u64) {
    if sz >= 1024 * 1024 {
        print!("{}M", sz / (1024 * 1024));
    } else if sz >= 1024 {
        print!("{}K", sz / 1024);
    } else {
        print!("{}B", sz);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut numeric_mode = false;
    let mut verbose_level = 0usize;
    let mut tree_mode = false;
    let mut filter_bus: Option<u8> = None;
    let mut filter_slot: Option<u8> = None;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: lspci [ОПЦИИ]...\n\
                     Вывод подробного списка всех устройств на шине PCI.\n\n\
                     Опции:\n  \
                       -n             Выводить числовые коды производителей и устройств (vendor:device)\n  \
                       -v             Подробный вывод (выводить адреса BAR, размеры и прерывания)\n  \
                       -vv            Очень подробный вывод (выводить сведения о драйвере ядра)\n  \
                       -t             Древовидный вывод топологии шины\n  \
                       -s [[ШИНА]:]СЛОТ[.ФУНКЦИЯ]  Фильтровать устройства по адресу\n  \
                       -h, --help     Показать эту справку и выйти\n  \
                       -V, --version  Вывести информацию о версии и выйти",
                    "Usage: lspci [OPTIONS]...\n\
                     List all PCI devices in the system.\n\n\
                     Options:\n  \
                       -n             Show numeric vendor and device codes\n  \
                       -v             Be verbose (display BAR addresses, sizes and IRQs)\n  \
                       -vv            Be very verbose (display kernel driver info)\n  \
                       -t             Show bus topology tree view\n  \
                       -s [[bus]:]slot[.func]  Show only devices in selected slots\n  \
                       -h, --help     Display this help and exit\n  \
                       -V, --version  Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("lspci (Wunix pciutils) 3.10.0");
            exit(0);
        } else if arg == b"-n" {
            numeric_mode = true;
        } else if arg == b"-v" {
            verbose_level = 1;
        } else if arg == b"-vv" {
            verbose_level = 2;
        } else if arg == b"-t" {
            tree_mode = true;
        } else if arg == b"-s" && i + 1 < argc {
            i += 1;
            let val = cstr_to_slice(unsafe { *argv.add(i as usize) });
            if let Ok(s) = core::str::from_utf8(val) {
                if let Some(pos) = s.find(':') {
                    filter_bus = parse_hex(&s[..pos]).map(|v| v as u8);
                    filter_slot = parse_hex(&s[pos + 1..]).map(|v| v as u8);
                } else {
                    filter_slot = parse_hex(s).map(|v| v as u8);
                }
            }
        }
        i += 1;
    }

    let mut buf = [0u8; 8192];
    let file_len = read_proc_pci_devices(&mut buf);
    let content = core::str::from_utf8(&buf[..file_len]).unwrap_or("");
    let mut entries: [Option<PciDevEntry>; 32] = [None; 32];
    let count = load_pci_devices(content, &mut entries);

    if count == 0 {
        println!("{}", t("Устройства PCI не обнаружены.", "No PCI devices found."));
        exit(0);
    }

    if tree_mode {
        println!("-[0000:00]---");
        for idx in 0..count {
            if let Some(e) = entries[idx] {
                let vend_name = pci_vendor_name(e.vendor_id);
                let dev_name = pci_device_desc(e.vendor_id, e.device_id);
                println!("           +-{:02x}.{}  {} {}", e.dev, e.func, vend_name, dev_name);
            }
        }
        exit(0);
    }

    for idx in 0..count {
        let e = match entries[idx] {
            Some(entry) => entry,
            None => continue,
        };

        if let Some(b) = filter_bus {
            if e.bus != b {
                continue;
            }
        }
        if let Some(s) = filter_slot {
            if e.dev != s {
                continue;
            }
        }

        let (class_name, full_class) = pci_class_name(e.class_code, e.subclass);
        let vend_name = pci_vendor_name(e.vendor_id);
        let dev_name = pci_device_desc(e.vendor_id, e.device_id);

        if numeric_mode {
            println!(
                "{:02x}:{:02x}.{} {:04x}: {:04x}:{:04x}",
                e.bus, e.dev, e.func, full_class, e.vendor_id, e.device_id
            );
        } else {
            println!(
                "{:02x}:{:02x}.{} {}: {} {}",
                e.bus, e.dev, e.func, class_name, vend_name, dev_name
            );
        }

        if verbose_level >= 1 {
            if e.irq > 0 {
                println!("\t{}: {}", t("Прерывание", "Interrupt"), e.irq);
            }

            for bar_idx in 0..6 {
                let addr = e.bars[bar_idx];
                let sz = e.bar_sizes[bar_idx];
                if addr > 0 && sz > 0 {
                    if (addr & 1) != 0 || addr < 0x10000 {
                        print!("\tI/O ports at {:04x} [size=", addr & !1);
                        print_size(sz);
                        println!("]");
                    } else {
                        print!("\tMemory at {:08x} (32-bit, non-prefetchable) [size=", addr & !0xF);
                        print_size(sz);
                        println!("]");
                    }
                }
            }

            if verbose_level >= 2 && e.driver != "-" {
                println!("\t{} {}", t("Используемый драйвер ядра:", "Kernel driver in use:"), e.driver);
            }
        }
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
