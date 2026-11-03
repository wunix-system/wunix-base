#![no_std]
#![no_main]

mod args;
mod exec;

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

    match args::ShutdownArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            let code = exec::execute(parsed);
            exit(code as u64);
        }
        Err("help") => {
            print_help();
            exit(0);
        }
        Err("version") => {
            println!("shutdown (Wunix systemd) 255");
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
            "Использование: shutdown [ОПЦИИ]... [ВРЕМЯ] [СООБЩЕНИЕ...]\n\
             Завершение работы, выключение или перезагрузка системы.\n\n\
             Опции:\n  \
               -h, -P, --poweroff     Выключить питание компьютера (по умолчанию)\n  \
               -r, --reboot           Перезагрузить компьютер\n  \
               -H, --halt             Остановить систему без выключения питания\n  \
               -c, --cancel           Отменить ранее запланированное выключение\n  \
               -k                     Только разослать предупреждения без реального выключения\n  \
               --no-wall              Не отправлять широковещательное сообщение пользователям\n  \
               --help                 Показать эту справку и выйти\n  \
               -V, --version          Вывести информацию о версии и выйти\n\n\
             Спецификация времени:\n  \
               now, +0                Немедленно\n  \
               +m                     Через m минут (например: +5)\n  \
               hh:mm                  В указанное время суток (24ч формат)",
            "Usage: shutdown [OPTIONS]... [TIME] [WALL...]\n\
             Shut down, power-off or reboot the machine.\n\n\
             Options:\n  \
               -h, -P, --poweroff     Power-off the machine (default)\n  \
               -r, --reboot           Reboot the machine\n  \
               -H, --halt             Halt the machine\n  \
               -c, --cancel           Cancel a pending shutdown\n  \
               -k                     Only warn, don't actually shut down\n  \
               --no-wall              Don't send a wall message\n  \
               --help                 Show this help and exit\n  \
               -V, --version          Print version information\n\n\
             Time specifications:\n  \
               now, +0                Immediately\n  \
               +m                     In m minutes from now (e.g. +5)\n  \
               hh:mm                  At specific clock time"
        )
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
