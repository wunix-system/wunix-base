use libwunix::time::{clock_realtime, get_system_timezone, sleep_sec, DateTime};
use libwunix::{println, sync, system_halt, system_poweroff, system_reboot, t};
use crate::args::{ShutdownArgs, ShutdownMode};

pub fn execute(args: ShutdownArgs) -> i32 {
    // 1. Отмена запланированного выключения
    if args.cancel {
        println!(
            "{}",
            t(
                "Запланированное выключение системы отменено администратором.",
                "Scheduled system shutdown has been cancelled by administrator."
            )
        );
        return 0;
    }

    // 2. Рассылка широковещательного сообщения (Wall notification)
    if !args.no_wall {
        broadcast_wall_message(&args);
    }

    // 3. Режим имитации (-k): только отправка предупреждений
    if args.warn_only {
        println!(
            "{}",
            t(
                "shutdown: режим проверки (-k): реальное выключение не выполняется.",
                "shutdown: test mode (-k): actual shutdown omitted."
            )
        );
        return 0;
    }

    // 4. Ожидание назначенного таймера
    if args.delay_secs > 0 {
        let mins = (args.delay_secs + 59) / 60;
        println!(
            "{} {} {}.",
            t("Система будет остановлена через", "The system will shut down in"),
            mins,
            t("мин", "min")
        );
        sleep_sec(args.delay_secs);
    }

    // 5. Синхронизация дисковых буферов
    println!("{}", t("Синхронизация файловых систем (sync)...", "Flushing filesystem buffers (sync)..."));
    sync();

    // 6. Вызов системного вызова управления питанием ядра
    let res = match args.mode {
        ShutdownMode::PowerOff => system_poweroff(),
        ShutdownMode::Reboot => system_reboot(),
        ShutdownMode::Halt => system_halt(),
    };

    if res < 0 {
        println!(
            "{}",
            t(
                "shutdown: ошибка: требуются привилегии суперпользователя (root).",
                "shutdown: error: operation not permitted (root privileges required)."
            )
        );
        1
    } else {
        0
    }
}

fn get_user_and_host<'a>(host_buf: &'a mut [u8]) -> (&'static str, &'a str) {
    let user = libwunix::env::getenv("USER").unwrap_or("user");
    if let Some(host) = libwunix::env::getenv("HOSTNAME") {
        return (user, host);
    }
    let fd = libwunix::open(b"/etc/hostname\0", 0, 0);
    if fd >= 0 {
        let n = libwunix::read(fd as u64, host_buf);
        let _ = libwunix::close(fd as u64);
        if n > 0 {
            if let Ok(h) = core::str::from_utf8(&host_buf[..n as usize]) {
                let trimmed = h.trim();
                if !trimmed.is_empty() {
                    return (user, trimmed);
                }
            }
        }
    }
    (user, "wunix")
}

fn broadcast_wall_message(args: &ShutdownArgs) {
    let (tz_offset, tz_abbr, _) = get_system_timezone();
    let now = clock_realtime();
    let dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, tz_offset);
    let mut time_buf = [0u8; 64];
    let time_s = dt.format_strftime("%a %F %T", tz_abbr, tz_offset, false, &mut time_buf);

    let mut host_buf = [0u8; 64];
    let (user, host) = get_user_and_host(&mut host_buf);

    println!();
    println!(
        "\x1b[1;31m*** {} {}@{} ({}) ***\x1b[0m",
        t("Широковещательное оповещение от", "Broadcast message from"),
        user,
        host,
        time_s
    );

    if let Some(msg) = args.wall_message {
        println!("{}", msg);
    } else {
        match args.mode {
            ShutdownMode::PowerOff => {
                println!(
                    "{}",
                    t(
                        "Система выключается по запросу администратора!",
                        "The system is powering down now!"
                    )
                );
            }
            ShutdownMode::Reboot => {
                println!(
                    "{}",
                    t(
                        "Система перезагружается по запросу администратора!",
                        "The system is rebooting now!"
                    )
                );
            }
            ShutdownMode::Halt => {
                println!(
                    "{}",
                    t(
                        "Система останавливается по запросу администратора!",
                        "The system is halting now!"
                    )
                );
            }
        }
    }
    println!();
}
