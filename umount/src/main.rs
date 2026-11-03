#![no_std]
#![no_main]

use libwunix::{exit, println, t, umount2};

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
    let mut target_dir: Option<&'static [u8]> = None;
    let mut verbose = false;
    let mut flags: i32 = 0;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: umount [ОПЦИИ] <каталог | устройство>\n\
                     Опции:\n  \
                       -f, --force     Принудительное размонтирование\n  \
                       -v, --verbose   Подробный вывод\n  \
                       -h, --help      Показать эту справку\n  \
                       -V, --version   Версия umount",
                    "Usage: umount [OPTIONS] <directory | device>\n\
                     Options:\n  \
                       -f, --force     Force unmount\n  \
                       -v, --verbose   Verbose output\n  \
                       -h, --help      Display this help and exit\n  \
                       -V, --version   Display version"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("umount (Wunix util-linux) 2.39.0");
            exit(0);
        } else if arg == b"-v" || arg == b"--verbose" {
            verbose = true;
        } else if arg == b"-f" || arg == b"--force" {
            flags |= 1; // MNT_FORCE
        } else if !arg.starts_with(b"-") && target_dir.is_none() {
            target_dir = Some(arg);
        }
        i += 1;
    }

    let target = match target_dir {
        Some(t) => t,
        None => {
            println!(
                "{}",
                t(
                    "umount: укажите точку монтирования (например, 'umount /mnt')",
                    "umount: mountpoint required (e.g. 'umount /mnt')"
                )
            );
            exit(1);
        }
    };

    let mut tgt_null = [0u8; 256];
    let t_len = target.len().min(255);
    tgt_null[..t_len].copy_from_slice(&target[..t_len]);

    let ret = umount2(&tgt_null[..t_len + 1], flags);
    let tgt_s = core::str::from_utf8(target).unwrap_or("");

    if ret < 0 {
        println!(
            "{}: {} (код ошибки: {})",
            t("umount: ошибка отмонтирования", "umount: unmount failed"),
            tgt_s,
            ret
        );
        exit(1);
    }

    if verbose {
        println!("{}: {}", t("Успешно отмонтировано", "Unmounted"), tgt_s);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("umount: panic occurred");
    exit(1);
}
