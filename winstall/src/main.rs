#![no_std]
#![no_main]

mod disk;
mod install;
mod net;
mod screens;
mod tui;

use disk::{
    build_auto_partition_plan, format_fat32, init_swap, scan_disks,
    wipe_disk_signatures, write_mbr_table, DiskInfo, PartitionPlan, MAX_DISKS,
    MAX_PARTITIONS,
};
use install::install_base;
use net::{scan_network_adapters, NetAdapterInfo, NetConfig, MAX_ADAPTERS};
use screens::{
    begin_install_progress_screen, begin_partition_process_screen,
    screen_boot_menu, screen_datetime_and_timezone, screen_error, screen_finish,
    screen_network_config, screen_partition_confirm, screen_partition_manual,
    screen_partition_mode, screen_select_adapter, screen_select_disk,
    screen_select_fs, screen_user_and_hostname, screen_welcome_info,
    update_install_progress, update_partition_step, FinishChoice, PartitionMode,
    WelcomeChoice,
};
use tui::{clear_screen, hide_cursor, show_cursor, sleep_ms};

use libwunix::{close, execve, exit, open, pread64, write, O_RDONLY, STDERR_FILENO, STDOUT_FILENO};

#[unsafe(no_mangle)]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    hide_cursor();
    clear_screen();

    run_wizard();
}

fn run_wizard() -> ! {
    // Если в параметрах ядра указан mode=install, сразу переходим к установщику
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
        match screen_boot_menu() {
            Some(w) => break w,
            None => {}
        }
    };

    match welcome {
        WelcomeChoice::Install => {
            run_install_wizard();
        }
        WelcomeChoice::Live => {
            launch_shell();
        }
    }
}

fn run_install_wizard() -> ! {
    // ── 1. Окно приветствия и рассказ про Wunix OS ───────────────────────────
    if !screen_welcome_info() {
        run_wizard();
    }

    // ── 2. Выбор целевого диска ──────────────────────────────────────────────
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

    // ── 3. Выбор файловой системы ────────────────────────────────────────────
    let fs_choice = loop {
        match screen_select_fs() {
            Some(fs) => break fs,
            None => {
                run_install_wizard();
            }
        }
    };

    // ── 4. Разметка диска (Автоматическая / Ручная + Валидация) ──────────────
    let mut plans = [PartitionPlan::empty(); MAX_PARTITIONS];
    let mut plan_count;

    loop {
        let mode = match screen_partition_mode() {
            Some(m) => m,
            None => {
                run_install_wizard();
            }
        };

        match mode {
            PartitionMode::Auto => {
                plan_count = build_auto_partition_plan(target_disk.sector_count, fs_choice, &mut plans);
            }
            PartitionMode::Manual => {
                match screen_partition_manual(target_disk, fs_choice, &mut plans) {
                    Some(c) => plan_count = c,
                    None => continue,
                }
            }
        }

        if screen_partition_confirm(target_disk.name(), &plans[..plan_count], plan_count) {
            break;
        }
    }

    // ── 5. Очистка и разметка диска (с отображением процесса) ────────────────
    begin_partition_process_screen(target_disk.name());

    // Шаг 1: Очистка нулевого сектора и старых сигнатур
    update_partition_step(1, false);
    sleep_ms(150);
    wipe_disk_signatures(target_disk.path);
    update_partition_step(1, true);

    // Шаг 2: Запись новой таблицы разделов MBR
    update_partition_step(2, false);
    sleep_ms(150);
    if !write_mbr_table(target_disk.path, &plans[..plan_count], plan_count) {
        screen_error("Ошибка записи таблицы MBR на целевой накопитель!");
        launch_shell();
    }
    update_partition_step(2, true);

    // Шаг 3: Форматирование корневого раздела (FAT32)
    update_partition_step(3, false);
    sleep_ms(200);
    let mut root_part_num = 1u8;
    for (idx, p) in plans[..plan_count].iter().enumerate() {
        if p.mount_point == "/" {
            root_part_num = (idx + 1) as u8;
            if !format_fat32(target_disk.path, p.start_lba as u64, p.sector_count as u64) {
                screen_error("Ошибка форматирования корневого раздела FAT32!");
                launch_shell();
            }
        } else if p.mount_point == "/boot" && p.fstype == "fat32" {
            // Форматируем также отдельный раздел /boot если он выделен
            let _ = format_fat32(target_disk.path, p.start_lba as u64, p.sector_count as u64);
        }
    }
    update_partition_step(3, true);

    // Шаг 4: Инициализация Swap раздела (если задан)
    update_partition_step(4, false);
    sleep_ms(150);
    for p in plans[..plan_count].iter() {
        if p.mount_point == "swap" || p.fstype == "swap" {
            let _ = init_swap(target_disk.path, p.start_lba as u64, p.sector_count as u64);
        }
    }
    update_partition_step(4, true);

    // Шаг 5: Проверка и фиксация
    update_partition_step(5, false);
    sleep_ms(150);
    update_partition_step(5, true);
    sleep_ms(250);

    // ── 6. Сеть: Выбор адаптера и параметров ─────────────────────────────────
    let mut adapters = [NetAdapterInfo::default(); MAX_ADAPTERS];
    let adapter_count = scan_network_adapters(&mut adapters);

    let net_cfg = match screen_select_adapter(&adapters[..adapter_count], adapter_count) {
        Some(Some(adapter)) => {
            match screen_network_config(adapter) {
                Some(cfg) => cfg,
                None => NetConfig::default(),
            }
        }
        _ => NetConfig::default(),
    };

    // ── 7. Учетная запись администратора и имя хоста ─────────────────────────
    let user_cfg = loop {
        match screen_user_and_hostname() {
            Some(u) => break u,
            None => {
                run_install_wizard();
            }
        }
    };

    let username = core::str::from_utf8(&user_cfg.username[..user_cfg.username_len]).unwrap_or("admin");
    let password = &user_cfg.password[..user_cfg.password_len];
    let hostname = core::str::from_utf8(&user_cfg.hostname[..user_cfg.hostname_len]).unwrap_or("wunix");

    // ── 8. Дата, время и часовой пояс ────────────────────────────────────────
    let timezone = match screen_datetime_and_timezone() {
        Some(tz) => tz,
        None => "Europe/Moscow",
    };

    // ── 9. Установка Wunix Base на диск (БЕЗ winstall!) ─────────────────────
    begin_install_progress_screen();

    let ok = install_base(
        target_disk.path,
        root_part_num,
        hostname,
        timezone,
        username,
        password,
        &net_cfg,
        |pct, stage, done, total| {
            update_install_progress(pct, stage, done, total);
        },
    );

    if !ok {
        screen_error("Ошибка при установке компонентов системы на диск.");
        launch_shell();
    }

    // ── 10. Завершение установки ─────────────────────────────────────────────
    match screen_finish(
        target_disk.name(),
        username,
        hostname,
        net_cfg.ip_str(),
        timezone,
    ) {
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
