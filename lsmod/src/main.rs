#![no_std]
#![no_main]

use libwunix::{close, exit, open, println, read, t};

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

fn parse_u64(s: &str) -> u64 {
    let mut val = 0u64;
    for b in s.bytes() {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u64;
        } else {
            break;
        }
    }
    val
}

#[derive(Clone, Copy)]
struct ModuleEntry<'a> {
    name: &'a str,
    size: u64,
    instances: u64,
    deps: &'a str,
    _state: &'a str,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut sort_by_size = false;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: lsmod [ОПЦИИ]...\n\
                     Вывод списка активных драйверов и модулей ядра Wunix.\n\n\
                     Опции:\n  \
                       -s, --sort     Сортировать модули по размеру (по убыванию)\n  \
                       -h, --help     Показать эту справку и выйти\n  \
                       -V, --version  Вывести информацию о версии и выйти",
                    "Usage: lsmod [OPTIONS]...\n\
                     List loaded kernel drivers and modules.\n\n\
                     Options:\n  \
                       -s, --sort     Sort modules by memory size\n  \
                       -h, --help     Display this help and exit\n  \
                       -V, --version  Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("lsmod (Wunix kmod) 31.0");
            exit(0);
        } else if arg == b"-s" || arg == b"--sort" {
            sort_by_size = true;
        }
        i += 1;
    }

    let mut buf = [0u8; 4096];
    let mut len = 0usize;
    let fd = open(b"/proc/modules\0", 0, 0);
    if fd >= 0 {
        let n = read(fd as u64, &mut buf);
        if n > 0 {
            len = n as usize;
        }
        close(fd as u64);
    }

    let text = core::str::from_utf8(&buf[..len]).unwrap_or("");
    let mut entries: [Option<ModuleEntry>; 32] = [None; 32];
    let mut count = 0usize;

    for line in text.lines() {
        if line.is_empty() || count >= 32 {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = match parts.next() { Some(n) => n, None => continue };
        let size_str = match parts.next() { Some(s) => s, None => continue };
        let inst_str = match parts.next() { Some(i) => i, None => continue };
        let deps = parts.next().unwrap_or("-");
        let state = parts.next().unwrap_or("Live");

        entries[count] = Some(ModuleEntry {
            name,
            size: parse_u64(size_str),
            instances: parse_u64(inst_str),
            deps,
            _state: state,
        });
        count += 1;
    }

    if count == 0 {
        println!("{}", t("Модули ядра не загружены.", "No kernel modules loaded."));
        exit(0);
    }

    if sort_by_size {
        // Сортировка по убыванию размера
        for i in 0..count {
            for j in 0..count - 1 - i {
                if let (Some(a), Some(b)) = (entries[j], entries[j + 1]) {
                    if a.size < b.size {
                        entries.swap(j, j + 1);
                    }
                }
            }
        }
    }

    println!(
        "{:<24} {:>8}  {:<3} {}",
        t("Модуль", "Module"),
        t("Размер", "Size"),
        t("Исп", "Used"),
        t("Зависимости / Кем используется", "by")
    );

    for idx in 0..count {
        if let Some(m) = entries[idx] {
            let deps_display = if m.deps == "-" { "" } else { m.deps };
            println!("{:<24} {:>8}  {:<3} {}", m.name, m.size, m.instances, deps_display);
        }
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
