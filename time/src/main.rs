#![no_std]
#![no_main]

mod args;
mod format;
mod runner;

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

    match args::TimeArgs::parse(&arg_slices[..count]) {
        Ok(parsed) => {
            match runner::execute(&parsed) {
                Ok(result) => {
                    format::output_results(&result, &parsed);
                    exit(result.exit_status as u64);
                }
                Err(code) => {
                    exit(code as u64);
                }
            }
        }
        Err("help") => {
            println!(
                "{}",
                t(
                    "Использование: time [ОПЦИИ]... КОМАНДА [АРГУМЕНТЫ]...\n\
                     Запустить указанную КОМАНДУ и измерить время её работы.\n\n\
                     Опции:\n  \
                       -p, --portability       Вывод в стандартном формате POSIX (real, user, sys)\n  \
                       -v, --verbose           Подробный отчет о времени и использовании ресурсов\n  \
                       -o, --output=ФАЙЛ       Записать результат в указанный файл вместо stderr\n  \
                       -a, --append            Дописывать в файл (вместе с -o)\n  \
                       -f, --format=ФОРМАТ     Пользовательский формат вывода (%E, %e, %x, %C)\n  \
                       -h, --help              Показать эту справку и выйти\n  \
                       -V, --version           Вывести информацию о версии и выйти",
                    "Usage: time [OPTION]... COMMAND [ARGS]...\n\
                     Run COMMAND and summarize system resource usage and execution duration.\n\n\
                     Options:\n  \
                       -p, --portability       Output in POSIX standard 1003.2 format\n  \
                       -v, --verbose           Detailed summary of command execution\n  \
                       -o, --output=FILE       Send output to FILE instead of stderr\n  \
                       -a, --append            Append to output file instead of overwriting\n  \
                       -f, --format=FORMAT     Custom format string (%E, %e, %x, %C)\n  \
                       -h, --help              Display this help and exit\n  \
                       -V, --version           Output version information and exit"
                )
            );
            exit(0);
        }
        Err("version") => {
            println!("time (Wunix coreutils) 1.9");
            exit(0);
        }
        Err("missing_command") => {
            println!(
                "{}",
                t(
                    "time: ошибка: не указана команда для выполнения. Попробуйте 'time --help'.",
                    "time: error: missing command to execute. Try 'time --help'."
                )
            );
            exit(1);
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
