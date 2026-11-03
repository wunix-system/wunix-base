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

fn parse_u32(s: &str) -> u32 {
    let mut val = 0u32;
    for b in s.bytes() {
        if (b'0'..=b'9').contains(&b) {
            val = val * 10 + (b - b'0') as u32;
        } else {
            break;
        }
    }
    val
}

fn read_sysfs_string<'a>(path: &[u8], buf: &'a mut [u8]) -> Option<&'a str> {
    let fd = open(path, 0, 0);
    if fd < 0 {
        return None;
    }
    let n = read(fd as u64, buf);
    close(fd as u64);
    if n > 0 {
        let s = core::str::from_utf8(&buf[..n as usize]).ok()?;
        Some(s.trim())
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    let mut show_info = false;
    let mut set_x: Option<u32> = None;
    let mut set_y: Option<u32> = None;
    let mut set_depth: Option<u32> = None;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: fbset [ОПЦИИ]...\n\
                     Отображение и настройка параметров кадрового буфера дисплея.\n\n\
                     Опции:\n  \
                       -i, --info      Показать подробную информацию об устройстве кадрового буфера\n  \
                       -x <ширина>     Задать горизонтальное разрешение (в пикселях)\n  \
                       -y <высота>     Задать вертикальное разрешение (в пикселях)\n  \
                       -depth <бит>    Задать цветовую глубину (16, 24, 32)\n  \
                       -h, --help      Показать эту справку и выйти\n  \
                       -V, --version   Вывести информацию о версии и выйти",
                    "Usage: fbset [OPTIONS]...\n\
                     Show and modify frame buffer device settings.\n\n\
                     Options:\n  \
                       -i, --info      Display full frame buffer device info\n  \
                       -x <width>      Set horizontal resolution in pixels\n  \
                       -y <height>     Set vertical resolution in pixels\n  \
                       -depth <bits>   Set color depth (16, 24, 32)\n  \
                       -h, --help      Display this help and exit\n  \
                       -V, --version   Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("fbset (Wunix fbutils) 2.1");
            exit(0);
        } else if arg == b"-i" || arg == b"--info" {
            show_info = true;
        } else if arg == b"-x" && i + 1 < argc {
            i += 1;
            let val = cstr_to_slice(unsafe { *argv.add(i as usize) });
            if let Ok(s) = core::str::from_utf8(val) {
                set_x = Some(parse_u32(s));
            }
        } else if arg == b"-y" && i + 1 < argc {
            i += 1;
            let val = cstr_to_slice(unsafe { *argv.add(i as usize) });
            if let Ok(s) = core::str::from_utf8(val) {
                set_y = Some(parse_u32(s));
            }
        } else if arg == b"-depth" && i + 1 < argc {
            i += 1;
            let val = cstr_to_slice(unsafe { *argv.add(i as usize) });
            if let Ok(s) = core::str::from_utf8(val) {
                set_depth = Some(parse_u32(s));
            }
        }
        i += 1;
    }

    let mut buf_modes = [0u8; 128];
    let modes_str = match read_sysfs_string(b"/sys/class/graphics/fb0/modes\0", &mut buf_modes) {
        Some(s) => s,
        None => {
            println!(
                "{}",
                t(
                    "fbset: ошибка: устройство кадрового буфера /dev/fb0 не найдено",
                    "fbset: error: framebuffer device /dev/fb0 not found"
                )
            );
            exit(1);
        }
    };

    let mut width = 1024u32;
    let mut height = 768u32;
    let mut depth = 32u32;

    // Парсим формат: "1024x768-32"
    let line = modes_str.lines().next().unwrap_or("");
    if let Some(x_pos) = line.find('x') {
        width = parse_u32(&line[..x_pos]);
        let rest = &line[x_pos + 1..];
        if let Some(dash_pos) = rest.find('-') {
            height = parse_u32(&rest[..dash_pos]);
            depth = parse_u32(&rest[dash_pos + 1..]);
        } else {
            height = parse_u32(rest);
        }
    }

    if let Some(w) = set_x { width = w; }
    if let Some(h) = set_y { height = h; }
    if let Some(d) = set_depth { depth = d; }

    let bytes_per_pixel = depth / 8;
    let mut line_length = width * bytes_per_pixel;

    let mut buf_stride = [0u8; 64];
    if let Some(stride_str) = read_sysfs_string(b"/sys/class/graphics/fb0/stride\0", &mut buf_stride) {
        let parsed_stride = parse_u32(stride_str);
        if parsed_stride > 0 {
            line_length = parsed_stride;
        }
    }

    let total_size = line_length * height;

    println!("mode \"{}x{}\"", width, height);
    println!("    geometry {} {} {} {} {}", width, height, width, height, depth);
    println!("    timings 0 0 0 0 0 0 0");
    println!("    accel false");
    println!("    rgba 8/16,8/8,8/0,8/24");
    println!("endmode");

    if show_info {
        let mut buf_name = [0u8; 128];
        let fb_name = read_sysfs_string(b"/sys/class/graphics/fb0/name\0", &mut buf_name)
            .unwrap_or("Linear Framebuffer");

        let mut buf_addr = [0u8; 64];
        let fb_addr = read_sysfs_string(b"/sys/class/graphics/fb0/address\0", &mut buf_addr)
            .unwrap_or("unknown");

        println!();
        println!("{}", t("Информация об устройстве кадрового буфера:", "Frame buffer device information:"));
        println!("    {:<12}: {}", t("Название", "Name"), fb_name);
        println!("    {:<12}: {}", t("Адрес буфера", "Address"), fb_addr);
        print!("    {:<12}: {} ", t("Размер буфера", "Size"), total_size);
        println!("({} MiB)", total_size / (1024 * 1024));
        println!("    {:<12}: PACKED PIXELS", t("Тип памяти", "Type"));
        println!("    {:<12}: TRUECOLOR", t("Формат цвета", "Visual"));
        println!("    {:<12}: {} {}", t("Шаг строки", "LineLength"), line_length, t("байт", "bytes"));
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
