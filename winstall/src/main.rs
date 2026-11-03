#![no_std]
#![no_main]

mod disk;
mod install;
mod net;
mod screens;
mod tui;

use disk::{format_fat32, scan_disks, write_mbr, DiskInfo, MAX_DISKS};
use install::install_base;
use screens::{
    begin_progress_screen, screen_confirm_partition, screen_create_user,
    screen_error, screen_finish, screen_hostname, screen_network_config,
    screen_select_disk, screen_select_fs, screen_welcome, update_progress,
    FinishChoice, WelcomeChoice,
};
use tui::{clear_screen, hide_cursor, show_cursor};

use libwunix::{close, execve, exit, open, pread64, write, O_RDONLY, STDERR_FILENO, STDOUT_FILENO};

#[unsafe(no_mangle)]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    hide_cursor();
    clear_screen();

    run_wizard();
}

fn run_wizard() -> ! {
    // Если в параметрах командной строки ядра указан mode=install,
    // сразу переходим к интерактивному установщику
    let mut cmdline_buf = [0u8; 256];
    let cmd_fd = open(b"/proc/cmdline\0", O_RDONLY as u64, 0);
    if cmd_fd >= 0 {
        let n = pread64(cmd_fd as u64, &mut cmdline_buf, 0);
        close(cmd_fd as u64);
        if n > 0 {
            let cmd_str = core::str::from_utf8(&cmdline_buf[..n as usize]).unwrap_or("");
            if cmd_str.contains("mode=install") {
                run_install_wizard();
            }
        }
    }

    let welcome = loop {
        match screen_welcome() {
            Some(w) => break w,
            None => {}
        }
    };

    match welcome {
        WelcomeChoice::Live => {
            launch_shell();
        }
        WelcomeChoice::Install => {
            run_install_wizard();
        }
    }
}

fn run_install_wizard() -> ! {
    // ── 1. Выбор диска ───────────────────────────────────────────────────────
    let mut disks = [DiskInfo::default(); MAX_DISKS];
    let disk_count = scan_disks(&mut disks);

    let target_disk = loop {
        match screen_select_disk(&disks, disk_count) {
            Some(d) => break d,
            None => {
                run_wizard();
            }
        }
    };

    // ── 2. Выбор файловой системы ────────────────────────────────────────────
    let _fs_choice = loop {
        match screen_select_fs() {
            Some(fs) => break fs,
            None => {
                run_install_wizard();
            }
        }
    };

    // ── 3. Подтверждение разметки ────────────────────────────────────────────
    if !screen_confirm_partition(target_disk.name()) {
        run_install_wizard();
    }

    // ── 4. Настройка сети (DHCP / Static IP + Ping проверка) ──────────────────
    let net_cfg = loop {
        match screen_network_config() {
            Some(cfg) => break cfg,
            None => {
                run_install_wizard();
            }
        }
    };

    // ── 5. Создание пользователя-администратора (sudo / wheel) ───────────────
    let user_cfg = loop {
        match screen_create_user() {
            Some(cfg) => break cfg,
            None => {
                run_install_wizard();
            }
        }
    };

    // ── 6. Имя хоста ─────────────────────────────────────────────────────────
    let (hostname_buf, hostname_len) = loop {
        match screen_hostname() {
            Some(h) => break h,
            None => {
                run_install_wizard();
            }
        }
    };

    let hostname = {
        core::str::from_utf8(&hostname_buf[..hostname_len]).unwrap_or("wunix")
    };
    let username = {
        core::str::from_utf8(&user_cfg.username[..user_cfg.username_len]).unwrap_or("admin")
    };
    let password = &user_cfg.password[..user_cfg.password_len];

    // ── 7. Установка Wunix Base на диск ──────────────────────────────────────
    begin_progress_screen();
    let total_items = install::total_install_items();

    update_progress(1, "\u{417}\u{430}\u{43f}\u{438}\u{441}\u{44c} MBR \u{440}\u{430}\u{437}\u{434}\u{435}\u{43b}\u{43e}\u{432}...", 0, total_items);
    let sectors = target_disk.sector_count;
    if !write_mbr(target_disk.path, sectors) {
        screen_error("\u{41e}\u{448}\u{438}\u{431}\u{43a}\u{430} \u{437}\u{430}\u{43f}\u{438}\u{441}\u{438} MBR \u{43d}\u{430} \u{434}\u{438}\u{441}\u{43a}.");
        launch_shell();
    }

    update_progress(3, "\u{424}\u{43e}\u{440}\u{43c}\u{430}\u{442}\u{438}\u{440}\u{43e}\u{432}\u{430}\u{43d}\u{438}\u{445} FAT32...", 0, total_items);
    if !format_fat32(target_disk.path, 2048, sectors.saturating_sub(2048)) {
        screen_error("\u{41e}\u{448}\u{438}\u{431}\u{43a}\u{430} \u{444}\u{43e}\u{440}\u{43c}\u{430}\u{442}\u{438}\u{440}\u{43e}\u{432}\u{430}\u{43d}\u{438}\u{44f} FAT32.");
        launch_shell();
    }

    let ok = install_base(
        target_disk.path,
        hostname,
        username,
        password,
        &net_cfg,
        |pct, stage, done, total| {
            update_progress(pct, stage, done, total);
        },
    );

    if !ok {
        screen_error("\u{41e}\u{448}\u{438}\u{431}\u{43a}\u{430} \u{43f}\u{440}\u{438} \u{443}\u{441}\u{442}\u{430}\u{43d}\u{43e}\u{432}\u{43a}\u{435} \u{441}\u{438}\u{441}\u{442}\u{435}\u{43c}\u{44b} \u{43d}\u{430} \u{434}\u{438}\u{441}\u{43a}.");
        launch_shell();
    }

    // ── 8. Завершение установки ──────────────────────────────────────────────
    match screen_finish(target_disk.name(), username, hostname, net_cfg.ip_str()) {
        FinishChoice::Reboot => do_reboot(),
        FinishChoice::Shutdown => do_shutdown(),
        FinishChoice::Shell => launch_shell(),
    }
}

fn do_shutdown() -> ! {
    show_cursor();
    clear_screen();
    write(
        STDOUT_FILENO,
        "\x1b[1;33mВыключение системы (poweroff)...\x1b[0m\n".as_bytes(),
    );
    let argv = [b"/bin/shutdown\0".as_ptr(), core::ptr::null()];
    let _ = execve(b"/bin/shutdown\0", argv.as_ptr() as u64, 0);
    exit(0);
}

fn launch_shell() -> ! {
    show_cursor();
    clear_screen();
    write(
        STDOUT_FILENO,
        "\x1b[1;36mДобро пожаловать в Wunix OS (Live-сессия)!\x1b[0m\n\n".as_bytes(),
    );
    let argv = [b"/bin/wsh\0".as_ptr(), core::ptr::null()];
    let _ = execve(b"/bin/wsh\0", argv.as_ptr() as u64, 0);
    write(STDERR_FILENO, b"winstall: failed to exec /bin/wsh\n");
    exit(1);
}

fn do_reboot() -> ! {
    show_cursor();
    clear_screen();
    write(
        STDOUT_FILENO,
        "\x1b[1;32mПерезагрузка системы...\x1b[0m\n".as_bytes(),
    );
    let argv = [b"/bin/reboot\0".as_ptr(), core::ptr::null()];
    let _ = execve(b"/bin/reboot\0", argv.as_ptr() as u64, 0);
    exit(0);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    show_cursor();
    write(STDERR_FILENO, b"\x1b[1;31m[winstall] PANIC\x1b[0m\n");
    exit(1);
}
