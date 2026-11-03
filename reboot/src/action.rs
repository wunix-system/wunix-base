use libwunix::time::{clock_realtime, get_system_timezone, DateTime};
use libwunix::{println, sync, system_halt, system_poweroff, system_reboot, t};
use crate::args::{RebootAction, RebootArgs};

pub fn run(args: RebootArgs) -> i32 {
    // 1. Широковещательное оповещение (Wall broadcast)
    if !args.no_wall {
        broadcast_reboot(&args);
    }

    // 2. Имитация (запись в wtmp / аудит без вызова сброса)
    if args.wtmp_only {
        println!("{}", t("reboot: запись события в системный журнал wtmp (режим -w).", "reboot: logged reboot event to wtmp (-w mode)."));
        return 0;
    }

    // 3. Синхронизация файловых систем (если не указаны -n или -f)
    if !args.no_sync && !args.force {
        println!("{}", t("Синхронизация файловых систем (sync)...", "Flushing filesystem buffers (sync)..."));
        sync();
    }

    // 4. Вызов системного вызова перезапуска ядра
    let res = match args.action {
        RebootAction::Reboot => system_reboot(),
        RebootAction::Poweroff => system_poweroff(),
        RebootAction::Halt => system_halt(),
    };

    if res < 0 {
        println!(
            "{}",
            t(
                "reboot: ошибка: требуются права суперпользователя (root).",
                "reboot: error: operation not permitted (root required)."
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

fn broadcast_reboot(args: &RebootArgs) {
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

    match args.action {
        RebootAction::Reboot => {
            println!(
                "{}",
                t(
                    "Система перезагружается немедленно!",
                    "The system is rebooting now!"
                )
            );
        }
        RebootAction::Poweroff => {
            println!(
                "{}",
                t(
                    "Система выключает питание немедленно!",
                    "The system is powering down now!"
                )
            );
        }
        RebootAction::Halt => {
            println!(
                "{}",
                t(
                    "Система останавливается немедленно!",
                    "The system is halting now!"
                )
            );
        }
    }
    println!();
}
