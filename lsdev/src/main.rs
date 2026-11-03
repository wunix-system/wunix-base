#![no_std]
#![no_main]

mod collect;

use collect::{collect_devices, read_file, DeviceEntry};
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
    let mut show_irq = false;
    let mut show_io = false;
    let mut show_mmio = false;
    let mut show_drivers = false;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: lsdev [ОПЦИИ]...\n\
                     Динамическое отображение аппаратных устройств, привязанных драйверов и распределения ресурсов.\n\n\
                     Опции:\n  \
                       --irq          Показать только распределение линий прерываний (IRQ)\n  \
                       --io           Показать только адреса портов ввода-вывода (I/O)\n  \
                       --mmio         Показать только диапазоны адресов памяти MMIO\n  \
                       --drivers      Показать только сопоставление устройств и драйверов ядра\n  \
                       -h, --help     Показать эту справку и выйти\n  \
                       -V, --version  Вывести информацию о версии и выйти",
                    "Usage: lsdev [OPTIONS]...\n\
                     Display hardware devices, bound drivers and allocated resources.\n\n\
                     Options:\n  \
                       --irq          Show only IRQ allocations\n  \
                       --io           Show only I/O port assignments\n  \
                       --mmio         Show only MMIO address spaces\n  \
                       --drivers      Show only device-to-driver bindings\n  \
                       -h, --help     Display this help and exit\n  \
                       -V, --version  Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("lsdev (Wunix procinfo) 2.0.0");
            exit(0);
        } else if arg == b"--irq" {
            show_irq = true;
        } else if arg == b"--io" {
            show_io = true;
        } else if arg == b"--mmio" {
            show_mmio = true;
        } else if arg == b"--drivers" {
            show_drivers = true;
        }
        i += 1;
    }

    let mut buf = [0u8; 4096];

    if show_irq {
        let text = read_file(b"/proc/interrupts\0", &mut buf);
        if !text.is_empty() {
            println!("{}", text.trim_end());
        } else {
            println!("{}", t("Файл /proc/interrupts недоступен", "/proc/interrupts not available"));
        }
        exit(0);
    }

    if show_io {
        let text = read_file(b"/proc/ioports\0", &mut buf);
        if !text.is_empty() {
            println!("{}", text.trim_end());
        } else {
            println!("{}", t("Файл /proc/ioports недоступен", "/proc/ioports not available"));
        }
        exit(0);
    }

    if show_mmio {
        let text = read_file(b"/proc/iomem\0", &mut buf);
        if !text.is_empty() {
            println!("{}", text.trim_end());
        } else {
            println!("{}", t("Файл /proc/iomem недоступен", "/proc/iomem not available"));
        }
        exit(0);
    }

    let mut ioports_buf = [0u8; 2048];
    let ioports_text = read_file(b"/proc/ioports\0", &mut ioports_buf);

    let mut iomem_buf = [0u8; 2048];
    let iomem_text = read_file(b"/proc/iomem\0", &mut iomem_buf);

    let mut irq_buf = [0u8; 1024];
    let irq_text = read_file(b"/proc/interrupts\0", &mut irq_buf);

    let mut devices = [DeviceEntry::empty(); 16];
    let dev_count = collect_devices(ioports_text, iomem_text, irq_text, &mut devices);

    if show_drivers {
        println!(
            "{}",
            t(
                "Устройство       Драйвер ядра    Статус",
                "Device           Kernel Driver   Status"
            )
        );
        println!("-----------------------------------------------");
        for i in 0..dev_count {
            let dev = &devices[i];
            let n = core::str::from_utf8(&dev.name[..dev.name_len]).unwrap_or("?");
            let d = core::str::from_utf8(&dev.driver[..dev.driver_len]).unwrap_or("?");
            let st = t(dev.status_ru, dev.status);
            println!("{:<16} {:<15} {}", n, d, st);
        }
        exit(0);
    }

    println!(
        "{}",
        t(
            "Устройство       Драйвер         IRQ    Порты I/O        Память MMIO         Статус",
            "Device           Driver          IRQ    I/O Ports        MMIO Memory         Status"
        )
    );
    println!("--------------------------------------------------------------------------------------");

    for i in 0..dev_count {
        let dev = &devices[i];
        let n = core::str::from_utf8(&dev.name[..dev.name_len]).unwrap_or("?");
        let d = core::str::from_utf8(&dev.driver[..dev.driver_len]).unwrap_or("?");
        let irq_str = if dev.irq_len > 0 {
            core::str::from_utf8(&dev.irq[..dev.irq_len]).unwrap_or("-")
        } else {
            "-"
        };
        let io_str = if dev.io_len > 0 {
            core::str::from_utf8(&dev.io[..dev.io_len]).unwrap_or("-")
        } else {
            "-"
        };
        let mmio_str = if dev.mmio_len > 0 {
            core::str::from_utf8(&dev.mmio[..dev.mmio_len]).unwrap_or("-")
        } else {
            "-"
        };
        let st = t(dev.status_ru, dev.status);

        println!(
            "{:<16} {:<15} {:<6} {:<16} {:<19} {}",
            n, d, irq_str, io_str, mmio_str, st
        );
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
