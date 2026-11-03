#![no_std]
#![no_main]

mod action;
mod args;

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
    if argc > 0 {
        let envp = unsafe { argv.add(argc as usize + 1) };
        libwunix::env::init(envp);
    }

    let mut arg_slices: [&[u8]; 32] = [&[]; 32];
    let count = if argc > 1 {
        let n = ((argc - 1) as usize).min(32);
        for i in 0..n {
            arg_slices[i] = cstr_to_slice(unsafe { *argv.add(i + 1) });
        }
        n
    } else {
        0
    };

    match args::DateArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            let code = action::run(parsed);
            exit(code as u64);
        }
        Err("help") => {
            println!(
                "{}",
                t(
                    "Использование: date [ОПЦИИ]... [+ФОРМАТ]\n\
                     Вывести текущие дату и время в заданном ФОРМАТЕ или установить их.\n\n\
                     Опции:\n  \
                       -d, --date=СТРОКА      Вывести время, описанное СТРОКОЙ (@ts, YYYY-MM-DD HH:MM:SS)\n  \
                       -s, --set=СТРОКА       Установить системное время из СТРОКИ\n  \
                       -u, --utc, --universal Использовать шкалу всемирного времени UTC\n  \
                       -R, --rfc-email        Вывод в формате RFC 5322 (например, Fri, 30 Oct 2026 13:00:23 +0300)\n  \
                       -I, --iso-8601[=FMT]   Вывод в формате ISO 8601 (date, hours, minutes, seconds, ns)\n  \
                       -r, --reference=ФАЙЛ   Вывести время модификации указанного файла\n  \
                       -h, --help             Показать эту справку и выйти\n  \
                       -V, --version          Вывести информацию о версии и выйти",
                    "Usage: date [OPTION]... [+FORMAT]\n\
                     Display date and time in the given FORMAT, or set system time.\n\n\
                     Options:\n  \
                       -d, --date=STRING      Display time described by STRING (@ts, YYYY-MM-DD HH:MM:SS)\n  \
                       -s, --set=STRING       Set system time described by STRING\n  \
                       -u, --utc, --universal Output or set Universal Coordinated Time (UTC)\n  \
                       -R, --rfc-email        Output date in RFC 5322 format\n  \
                       -I, --iso-8601[=FMT]   Output date/time in ISO 8601 format\n  \
                       -r, --reference=FILE   Display the last modification time of FILE\n  \
                       -h, --help             Display this help and exit\n  \
                       -V, --version          Output version information and exit"
                )
            );
            exit(0);
        }
        Err("version") => {
            println!("date (Wunix coreutils) 9.4");
            exit(0);
        }
        Err(_) => {
            exit(1);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
