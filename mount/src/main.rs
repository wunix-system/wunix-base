#![no_std]
#![no_main]

use libwunix::{close, exit, mount, open, print, println, read, t};

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

fn print_mounts() {
    let fd = open(b"/proc/mounts\0", 0, 0);
    if fd < 0 {
        println!("{}", t("rootfs / rootfs rw 0 0", "rootfs / rootfs rw 0 0"));
        return;
    }
    let mut buf = [0u8; 4096];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n > 0 {
        if let Ok(s) = core::str::from_utf8(&buf[..n as usize]) {
            print!("{}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut fstype = &b"fat32"[..];
    let mut verbose = false;
    let mut positional = [&b""[..]; 2];
    let mut pos_count = 0;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование:\n  \
                       mount [-t тип] [-o опции] <источник> <каталог>\n  \
                       mount (без параметров: список точек монтирования)\n\
                     Опции:\n  \
                       -t, --types <тип>   Тип файловой системы (fat, fat32, vfat)\n  \
                       -o, --options <опц> Дополнительные опции монтирования (ro, rw)\n  \
                       -v, --verbose       Подробный вывод\n  \
                       -h, --help          Показать эту справку\n  \
                       -V, --version       Версия mount",
                    "Usage:\n  \
                       mount [-t fstype] [-o options] <source> <target>\n  \
                       mount (no arguments: list mount points)\n\
                     Options:\n  \
                       -t, --types <type>  Filesystem type (fat, fat32, vfat)\n  \
                       -o, --options <opt> Mount options (ro, rw)\n  \
                       -v, --verbose       Verbose output\n  \
                       -h, --help          Display this help and exit\n  \
                       -V, --version       Display version"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("mount (Wunix util-linux) 2.39.0");
            exit(0);
        } else if arg == b"-v" || arg == b"--verbose" {
            verbose = true;
        } else if arg == b"-t" || arg == b"--types" {
            i += 1;
            if i < argc {
                fstype = cstr_to_slice(unsafe { *argv.add(i as usize) });
            }
        } else if arg.starts_with(b"-t") && arg.len() > 2 {
            fstype = &arg[2..];
        } else if arg == b"-o" || arg == b"--options" {
            i += 1; // Пропускаем строку опций
        } else if !arg.starts_with(b"-") && pos_count < 2 {
            positional[pos_count] = arg;
            pos_count += 1;
        }
        i += 1;
    }

    if pos_count == 0 {
        print_mounts();
        exit(0);
    }

    if pos_count < 2 {
        println!(
            "{}",
            t(
                "mount: укажите источник и точку монтирования (например, 'mount -t fat /dev/vda1 /mnt')",
                "mount: source and target directory required (e.g. 'mount -t fat /dev/vda1 /mnt')"
            )
        );
        exit(1);
    }

    let source = positional[0];
    let target = positional[1];

    let mut src_null = [0u8; 256];
    let s_len = source.len().min(255);
    src_null[..s_len].copy_from_slice(&source[..s_len]);

    let mut tgt_null = [0u8; 256];
    let t_len = target.len().min(255);
    tgt_null[..t_len].copy_from_slice(&target[..t_len]);

    let mut fs_null = [0u8; 64];
    let f_len = fstype.len().min(63);
    fs_null[..f_len].copy_from_slice(&fstype[..f_len]);

    let ret = mount(&src_null[..s_len + 1], &tgt_null[..t_len + 1], &fs_null[..f_len + 1], 0, b"\0");

    let src_s = core::str::from_utf8(source).unwrap_or("");
    let tgt_s = core::str::from_utf8(target).unwrap_or("");
    let fs_s = core::str::from_utf8(fstype).unwrap_or("fat");

    if ret < 0 {
        println!(
            "{}: {} -> {} (код ошибки: {})",
            t("mount: ошибка монтирования", "mount: mount failed"),
            src_s,
            tgt_s,
            ret
        );
        exit(1);
    }

    if verbose {
        println!(
            "{} {} on {} ({})",
            t("Успешно смонтировано", "Mounted"),
            src_s,
            tgt_s,
            fs_s
        );
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("mount: panic occurred");
    exit(1);
}
