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

    match args::RebootArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            let code = action::run(parsed);
            exit(code as u64);
        }
        Err("help") => {
            print_help();
            exit(0);
        }
        Err("version") => {
            println!("reboot (Wunix systemd) 255");
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
            "Использование: reboot [ОПЦИИ]...\n\
             Перезагрузить или остановить систему Wunix.\n\n\
             Опции:\n  \
               -f, --force            Принудительный немедленный сброс без синхронизации\n  \
               -p, --poweroff         Выключить питание вместо перезапуска\n  \
               -d, --halt             Остановить систему вместо перезапуска\n  \
               -w, --wtmp-only        Только записать в wtmp, не перезагружать\n  \
               -n, --no-sync          Не синхронизировать файловые системы перед сбросом\n  \
               --no-wall              Не отправлять оповещение перед перезагрузкой\n  \
               -h, --help             Показать эту справку и выйти\n  \
               -V, --version          Вывести информацию о версии и выйти",
            "Usage: reboot [OPTIONS]...\n\
             Reboot, halt or power-off the system.\n\n\
             Options:\n  \
               -f, --force            Force immediate reboot without syncing\n  \
               -p, --poweroff         Power off instead of reboot\n  \
               -d, --halt             Halt instead of reboot\n  \
               -w, --wtmp-only        Only write wtmp record, don't reboot\n  \
               -n, --no-sync          Don't sync disks before reboot\n  \
               --no-wall              Don't send wall message before reboot\n  \
               -h, --help             Show this help and exit\n  \
               -V, --version          Print version information"
        )
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
