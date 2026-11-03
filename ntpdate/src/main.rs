#![no_std]
#![no_main]

mod args;
mod sync;

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

    match args::NtpdateArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            let code = sync::run(parsed);
            exit(code as u64);
        }
        Err("help") => {
            print_help();
            exit(0);
        }
        Err("version") => {
            println!("ntpdate (Wunix Network Time Protocol) 4.2.8");
            exit(0);
        }
        Err(_) => {
            exit(1);
        }
    }
}

fn print_help() {
    println!(
        "{}",
        t(
            "Использование: ntpdate [ОПЦИИ]... [СЕРВЕР]...\n\
             Установить или проверить системное время по протоколу NTP/SNTP.\n\n\
             Опции:\n  \
               -q         Режим только запроса (не изменять системные часы)\n  \
               -b         Принудительно скорректировать часы ступенчато (step time)\n  \
               -u         Использовать непривилегированный исходящий UDP порт\n  \
               -s         Тихий режим работы\n  \
               -v         Подробный вывод процесса опроса серверов\n  \
               -h, --help Показать эту справку и выйти\n  \
               -V, --version Вывести информацию о версии и выйти\n\n\
             Если сервер не указан, опрашиваются серверы из /etc/ntp.conf\n\
             или общедоступный пул pool.ntp.org.",
            "Usage: ntpdate [OPTIONS]... [SERVER]...\n\
             Query or set system time using Network Time Protocol (NTP/SNTP).\n\n\
             Options:\n  \
               -q         Query only (do not set system clock)\n  \
               -b         Force time stepping (step time)\n  \
               -u         Use unprivileged UDP source port\n  \
               -s         Silent mode\n  \
               -v         Verbose output\n  \
               -h, --help Display this help and exit\n  \
               -V, --version Output version information and exit\n\n\
             If no server is specified, servers from /etc/ntp.conf\n\
             or public pool.ntp.org are used."
        )
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
