//! Системный демон периодической проверки обновлений WSP (wspd).
//!
//! Работает как системная служба Wunix, обеспечивая фоновый мониторинг
//! целостности и доступности обновлений ядра и системных библиотек.

#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod checker;
mod config;

use libwunix::time::sleep_sec;
use libwunix::{exit, println, t};
use allocator::ALLOCATOR;
use checker::check_system_updates;
use config::load_daemon_config;

fn cstr_to_str(ptr: *const u8) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 && len < 4096 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(ptr, len);
        core::str::from_utf8(slice).unwrap_or("")
    }
}

#[derive(Default)]
struct CliOptions {
    once: bool,
    daemon: bool,
    verbose: bool,
}

fn parse_args(argc: isize, argv: *const *const u8) -> Result<CliOptions, &'static str> {
    let mut opts = CliOptions::default();

    for i in 1..argc {
        let arg = cstr_to_str(unsafe { *argv.add(i as usize) });
        match arg {
            "-1" | "--once" => opts.once = true,
            "-d" | "--daemon" => opts.daemon = true,
            "-v" | "--verbose" => opts.verbose = true,
            "-h" | "--help" => return Err("help"),
            "-V" | "--version" => return Err("version"),
            _ => {}
        }
    }

    Ok(opts)
}

fn print_help() {
    println!("{}", t(
        "Использование: wspd [ОПЦИИ]...\n\
         Системный фоновый демон проверки обновлений WSP в ОС Wunix.\n\n\
         Опции:\n\
           -1, --once       Выполнить однократную проверку обновлений и выйти\n\
           -d, --daemon     Запуск в режиме непрерывной фоновой службы (по умолчанию)\n\
           -v, --verbose    Подробный вывод процесса проверки\n\
           -h, --help       Показать эту справку и выйти\n\
           -V, --version    Вывести информацию о версии и выйти\n\n\
         Конфигурация загружается из файла /etc/wsp/daemon.conf.",
        "Usage: wspd [OPTIONS]...\n\
         Wunix System Package background update check daemon.\n\n\
         Options:\n\
           -1, --once       Run a single update check and exit\n\
           -d, --daemon     Run continuously in daemon mode (default)\n\
           -v, --verbose    Verbose log output\n\
           -h, --help       Show this help message and exit\n\
           -V, --version    Show version information and exit\n\n\
         Configuration is loaded from /etc/wsp/daemon.conf."
    ));
}

fn write_pid_file() {
    let _ = libwunix::mkdir(b"/run\0", 0o755);
    let _ = libwunix::mkdir(b"/run/services\0", 0o755);
    let _ = libwunix::mkdir(b"/var\0", 0o755);
    let _ = libwunix::mkdir(b"/var/run\0", 0o755);

    let pid = libwunix::getpid();
    let s = alloc::format!("{}\n", pid);
    for p in &[
        b"/run/services/wspd.pid\0".as_ref(),
        b"/run/wspd.pid\0".as_ref(),
        b"/var/run/wspd.pid\0".as_ref(),
    ] {
        let fd = libwunix::open(*p, (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_TRUNC) as u64, 0o644);
        if fd >= 0 {
            let _ = libwunix::write(fd as u64, s.as_bytes());
            let _ = libwunix::close(fd as u64);
        }
    }
}

fn print_version() {
    println!("wspd (Wunix System Package Daemon) 1.1.0 (x86_64)");
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> ! {
    if argc > 0 {
        let envp = unsafe { argv.add(argc as usize + 1) };
        libwunix::env::init(envp);
    }

    let opts = match parse_args(argc, argv) {
        Ok(o) => o,
        Err("help") => {
            print_help();
            exit(0);
        }
        Err("version") => {
            print_version();
            exit(0);
        }
        Err(_) => {
            exit(1);
        }
    };

    let conf = load_daemon_config();

    if !conf.enabled {
        if opts.verbose {
            println!("wspd: служба отключена в конфигурационном файле /etc/wsp/daemon.conf");
        }
        exit(0);
    }

    // Сохраняем PID файл для супервизора system
    write_pid_file();

    if opts.once || !opts.daemon {
        if opts.verbose {
            println!("wspd: запуск проверки системных компонентов WSP...");
        }
        let _res = check_system_updates(opts.verbose);
        exit(0);
    }

    // Режим непрерывного демона
    if opts.verbose {
        println!("wspd: системная служба запущена (интервал: {} сек, проверка на старте: {})",
            conf.check_interval, conf.check_on_boot);
    }

    if conf.check_on_boot {
        let _ = check_system_updates(opts.verbose);
        ALLOCATOR.reset();
    }

    loop {
        sleep_sec(conf.check_interval as u64);
        let _ = check_system_updates(opts.verbose);
        ALLOCATOR.reset();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
