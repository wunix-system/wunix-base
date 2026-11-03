#![no_std]
#![no_main]

mod args;
mod service;

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

    let mut arg_slices: [&[u8]; 16] = [&[]; 16];
    let count = if argc > 1 {
        let n = ((argc - 1) as usize).min(16);
        for i in 0..n {
            arg_slices[i] = cstr_to_slice(unsafe { *argv.add(i + 1) });
        }
        n
    } else {
        0
    };

    match args::TimesyncdArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            let code = service::run(parsed);
            exit(code as u64);
        }
        Err("help") => {
            println!(
                "{}",
                t(
                    "Использование: timesyncd [ОПЦИИ]...\n\
                     Системная служба синхронизации сетевого времени Wunix TimeSync Daemon.\n\n\
                     Опции:\n  \
                       -1, --once     Выполнить синхронизацию один раз и выйти\n  \
                       -d, --daemon   Работать непрерывно в фоновом режиме (по умолчанию)\n  \
                       -v, --verbose  Подробный вывод процесса синхронизации\n  \
                       -h, --help     Показать эту справку и выйти\n  \
                       -V, --version  Вывести информацию о версии и выйти\n\n\
                     Конфигурация считывается из файла /etc/timesyncd.conf.",
                    "Usage: timesyncd [OPTIONS]...\n\
                     Wunix TimeSync Network Time Synchronization Daemon.\n\n\
                     Options:\n  \
                       -1, --once     Run synchronization once and exit\n  \
                       -d, --daemon   Run continuously in daemon mode (default)\n  \
                       -v, --verbose  Verbose logging of sync events\n  \
                       -h, --help     Display this help and exit\n  \
                       -V, --version  Output version information and exit\n\n\
                     Configuration is loaded from /etc/timesyncd.conf."
                )
            );
            exit(0);
        }
        Err("version") => {
            println!("wunix-timesyncd 1.0.0");
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
