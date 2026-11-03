#![no_std]
#![no_main]

mod config;
mod status;
mod timezones;

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

    let mut cmd: Option<&str> = None;
    let mut extra_val: Option<&str> = None;

    let mut i = 0;
    while i < count {
        let arg = arg_slices[i];
        let arg_s = core::str::from_utf8(arg).unwrap_or("");

        if arg == b"-h" || arg == b"--help" {
            print_help();
            exit(0);
        } else if arg == b"--version" {
            println!("timedatectl (Wunix systemd) 255");
            exit(0);
        } else if arg == b"--no-pager" || arg == b"--adjust-system-clock" {
            // Флаги совместимости
        } else if cmd.is_none() {
            cmd = Some(arg_s);
        } else if extra_val.is_none() {
            extra_val = Some(arg_s);
        }
        i += 1;
    }

    let code = match cmd {
        None | Some("status") => {
            status::show_status();
            0
        }
        Some("show") => {
            status::show_properties();
            0
        }
        Some("timesync-status") => {
            status::show_timesync_status();
            0
        }
        Some("list-timezones") => {
            timezones::list_timezones();
            0
        }
        Some("set-time") => {
            match extra_val {
                Some(time_str) => config::set_time(time_str),
                None => {
                    println!("{}", t("timedatectl: укажите строку времени для установки.", "timedatectl: missing time specification."));
                    1
                }
            }
        }
        Some("set-timezone") => {
            match extra_val {
                Some(tz) => config::set_timezone(tz),
                None => {
                    println!("{}", t("timedatectl: укажите имя часового пояса.", "timedatectl: missing timezone name."));
                    1
                }
            }
        }
        Some("set-local-rtc") => {
            let enabled = match extra_val {
                Some("1") | Some("yes") | Some("true") | Some("on") => true,
                _ => false,
            };
            config::set_local_rtc(enabled)
        }
        Some("set-ntp") => {
            let enabled = match extra_val {
                Some("0") | Some("no") | Some("false") | Some("off") => false,
                _ => true,
            };
            config::set_ntp(enabled)
        }
        Some(unknown) => {
            println!("{}: '{}'.", t("timedatectl: неизвестная команда", "timedatectl: unknown command"), unknown);
            1
        }
    };

    exit(code as u64);
}

fn print_help() {
    println!(
        "{}",
        t(
            "Использование: timedatectl [ОПЦИИ]... [КОМАНДА]\n\
             Управление системным временем, RTC и часовым поясом.\n\n\
             Команды:\n  \
               status                   Показать текущее время и статус (по умолчанию)\n  \
               show                     Показать свойства системы в формате KEY=VALUE\n  \
               set-time ВРЕМЯ           Установить системное время ('YYYY-MM-DD HH:MM:SS')\n  \
               set-timezone ЗОНА        Установить системный часовой пояс (в /etc/timezone)\n  \
               list-timezones           Вывести список известных часовых поясов\n  \
               set-local-rtc [0|1]      Управлять хранением местного времени в RTC\n  \
               set-ntp [0|1]            Включить или отключить службу синхронизации NTP\n  \
               timesync-status          Показать статус сетевой синхронизации времени\n\n\
             Опции:\n  \
               -h, --help               Показать эту справку и выйти\n  \
               --version                Вывести версию программы и выйти\n  \
               --no-pager               Не перенаправлять вывод в пейджер",
            "Usage: timedatectl [OPTIONS]... [COMMAND]\n\
             Query or change system time, RTC, and timezone settings.\n\n\
             Commands:\n  \
               status                   Show current time settings (default)\n  \
               show                     Show properties in KEY=VALUE format\n  \
               set-time TIME            Set system time ('YYYY-MM-DD HH:MM:SS')\n  \
               set-timezone ZONE        Set system timezone (writes /etc/timezone)\n  \
               list-timezones           Show known timezones\n  \
               set-local-rtc [0|1]      Control whether RTC is in local time\n  \
               set-ntp [0|1]            Control whether NTP time sync is enabled\n  \
               timesync-status          Show status of time synchronization service\n\n\
             Options:\n  \
               -h, --help               Show this help and exit\n  \
               --version                Print program version\n  \
               --no-pager               Do not pipe output into a pager"
        )
    );
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    exit(1);
}
