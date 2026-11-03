#![no_std]
#![no_main]

mod catalog;
mod live;

use catalog::find_driver;
use libwunix::{exit, print, println, t};
use live::query_live_module;

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
    let mut field_filter: Option<&'static [u8]> = None;
    let mut target_module: Option<&'static [u8]> = None;

    let mut i = 1;
    while i < argc {
        let arg = cstr_to_slice(unsafe { *argv.add(i as usize) });
        if arg == b"-h" || arg == b"--help" {
            println!(
                "{}",
                t(
                    "Использование: modinfo [ОПЦИИ] <имя_модуля>\n\
                     Вывод подробной информации и метаданных драйвера ядра.\n\n\
                     Опции:\n  \
                       -F <поле>       Вывести только значение указанного поля (filename, description, author, license, version, alias, depends, parm, status)\n  \
                       -d, --description  Вывести только описание драйвера\n  \
                       -a, --author       Вывести только автора драйвера\n  \
                       -l, --license      Вывести только лицензию\n  \
                       -n, --filename     Вывести только путь к файлу драйвера\n  \
                       -p, --parameters   Вывести только настраиваемые параметры\n  \
                       -h, --help         Показать эту справку и выйти\n  \
                       -V, --version      Вывести информацию о версии и выйти",
                    "Usage: modinfo [OPTIONS] <module_name>\n\
                     Display information about a kernel driver or module.\n\n\
                     Options:\n  \
                       -F <field>      Print only the specified field\n  \
                       -d, --description  Print only module description\n  \
                       -a, --author       Print only module author\n  \
                       -l, --license      Print only module license\n  \
                       -n, --filename     Print only filename\n  \
                       -p, --parameters   Print only parameters\n  \
                       -h, --help         Display this help and exit\n  \
                       -V, --version      Output version information and exit"
                )
            );
            exit(0);
        } else if arg == b"-V" || arg == b"--version" {
            println!("modinfo (Wunix kmod) 31.0");
            exit(0);
        } else if arg == b"-F" && i + 1 < argc {
            i += 1;
            field_filter = Some(cstr_to_slice(unsafe { *argv.add(i as usize) }));
        } else if arg == b"-d" || arg == b"--description" {
            field_filter = Some(b"description");
        } else if arg == b"-a" || arg == b"--author" {
            field_filter = Some(b"author");
        } else if arg == b"-l" || arg == b"--license" {
            field_filter = Some(b"license");
        } else if arg == b"-n" || arg == b"--filename" {
            field_filter = Some(b"filename");
        } else if arg == b"-p" || arg == b"--parameters" {
            field_filter = Some(b"parm");
        } else if !arg.starts_with(b"-") {
            target_module = Some(arg);
        }
        i += 1;
    }

    let mod_name = match target_module {
        Some(m) => m,
        None => {
            println!(
                "{}",
                t(
                    "modinfo: Ошибка: укажите имя модуля ядра (например: modinfo virtio_blk)",
                    "modinfo: ERROR: missing module name argument (e.g. modinfo virtio_blk)"
                )
            );
            println!("{}", t("Попробуйте 'modinfo --help' для получения справки.", "Try 'modinfo --help' for more information."));
            exit(1);
        }
    };

    let drv = match find_driver(mod_name) {
        Some(d) => d,
        None => {
            println!(
                "{}: {}",
                t("modinfo: ОШИБКА: модуль не найден", "modinfo: ERROR: Module not found"),
                core::str::from_utf8(mod_name).unwrap_or("?")
            );
            exit(1);
        }
    };

    let mut mod_buf = [0u8; 4096];
    let live_status = query_live_module(drv.name, &mut mod_buf);

    let desc = t(drv.description_ru, drv.description);
    let pms = t(drv.parm_ru, drv.parm);

    if let Some(field) = field_filter {
        if field == b"filename" {
            println!("{}", drv.filename);
        } else if field == b"description" {
            println!("{}", desc);
        } else if field == b"author" {
            println!("{}", drv.author);
        } else if field == b"license" {
            println!("{}", drv.license);
        } else if field == b"version" {
            println!("{}", drv.version);
        } else if field == b"alias" {
            println!("{}", drv.alias);
        } else if field == b"depends" {
            println!("{}", drv.depends);
        } else if field == b"parm" {
            println!("{}", pms);
        } else if field == b"status" {
            if live_status.is_live {
                print!("Live (instances: {}, memory: {} bytes)", live_status.instances, live_status.size);
                if live_status.deps != "-" {
                    print!(", used by: {}", live_status.deps);
                }
                println!();
            } else {
                println!("{}", t("Не загружен", "Not loaded"));
            }
        } else {
            println!();
        }
        exit(0);
    }

    println!("{:<16} {}", "filename:", drv.filename);
    println!("{:<16} {}", "description:", desc);
    println!("{:<16} {}", "author:", drv.author);
    println!("{:<16} {}", "license:", drv.license);
    println!("{:<16} {}", "version:", drv.version);
    for line in drv.alias.lines() {
        println!("{:<16} {}", "alias:", line);
    }
    println!("{:<16} {}", "depends:", drv.depends);

    print!("{:<16} ", "status:");
    if live_status.is_live {
        print!(
            "{}",
            t(
                "В работе (экземпляров: ",
                "Live (instances: "
            )
        );
        print!("{}, ", live_status.instances);
        print!(
            "{}",
            t(
                "память: ",
                "memory: "
            )
        );
        print!("{} ", live_status.size);
        print!(
            "{}",
            t(
                "байт)",
                "bytes)"
            )
        );
        if live_status.deps != "-" {
            print!(
                " [{}: {}]",
                t("кем используется", "used by"),
                live_status.deps
            );
        }
        println!();
    } else {
        println!("{}", t("Не загружен", "Not loaded"));
    }

    for line in pms.lines() {
        println!("{:<16} {}", "parm:", line);
    }

    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
