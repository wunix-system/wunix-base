// Отрисовка TUI-экранов установщика Wunix OS.
// Принцип: абсолютное позиционирование (1..23), без скроллинга, без прыжков!

use crate::disk::DiskInfo;
use crate::net::{detect_current_network, parse_ipv4, ping_target_probes, NetConfig};
use crate::tui::{
    clear_screen, draw_box, draw_centered, draw_footer, draw_header, draw_info,
    draw_input_field, draw_label, draw_logo, draw_password_field, draw_progress,
    draw_separator, draw_status, goto, hide_cursor, input_password, input_text,
    run_menu, run_menu_compact, u64_to_buf, wait_key, write_u64, wr, wrs, MenuItem, BOLD,
    DIM, FG_CYAN, FG_GRAY, FG_GREEN, FG_RED, FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
};

// ─── Экран 1: TUI Загрузочное меню (Boot Menu) ────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WelcomeChoice {
    Live,
    Install,
}

pub fn screen_welcome() -> Option<WelcomeChoice> {
    hide_cursor();
    clear_screen();
    draw_header("  \u{417}\u{410}\u{413}\u{420}\u{423}\u{417}\u{41e}\u{427}\u{41d}\u{41e}\u{415} \u{41c}\u{415}\u{41d}\u{42e} WUNIX OS  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "\u{414}\u{43e}\u{431}\u{440}\u{43e} \u{43f}\u{43e}\u{436}\u{430}\u{43b}\u{43e}\u{432}\u{430}\u{442}\u{44c} \u{432} \u{43e}\u{43f}\u{435}\u{440}\u{430}\u{446}\u{438}\u{43e}\u{43d}\u{43d}\u{443}\u{44e} \u{441}\u{438}\u{441}\u{442}\u{435}\u{43c}\u{443} Wunix!");
    draw_centered(10, "\u{412}\u{44b}\u{431}\u{435}\u{440}\u{438}\u{442}\u{435} \u{440}\u{435}\u{436}\u{438}\u{43c} \u{437}\u{430}\u{433}\u{440}\u{443}\u{437}\u{43a}\u{438}:");

    let items = [
        MenuItem {
            label: "1.  \u{417}\u{430}\u{43f}\u{443}\u{441}\u{43a} Live ISO   (\u{437}\u{430}\u{43f}\u{443}\u{441}\u{43a} \u{432} RAM \u{434}\u{43b}\u{44f} \u{442}\u{435}\u{441}\u{442}\u{438}\u{440}\u{43e}\u{432}\u{430}\u{43d}\u{438}\u{44f})        ",
            disabled: false,
        },
        MenuItem {
            label: "2.  \u{423}\u{441}\u{442}\u{430}\u{43d}\u{43e}\u{432}\u{43a}\u{430} \u{441}\u{438}\u{441}\u{442}\u{435}\u{43c}\u{44b} (\u{443}\u{441}\u{442}\u{430}\u{43d}\u{43e}\u{432}\u{43a}\u{430} Wunix OS \u{43d}\u{430} \u{436}\u{451}\u{441}\u{442}\u{43a}\u{438}\u{439} \u{434}\u{438}\u{441}\u{43a}) ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" \u{2191}\u{2193} / 1-2 \u{412}\u{44b}\u{431}\u{43e}\u{440}    Enter / Space \u{417}\u{430}\u{43f}\u{443}\u{441}\u{43a} ");

    let sel = run_menu(12, 10, &items, 60)?;
    Some(if sel == 0 {
        WelcomeChoice::Live
    } else {
        WelcomeChoice::Install
    })
}

// ─── Экран 2: Выбор диска ────────────────────────────────────────────────────

pub fn screen_select_disk<'a>(disks: &'a [DiskInfo], count: usize) -> Option<&'a DiskInfo> {
    if count == 0 {
        screen_error("\u{41d}\u{435} \u{43d}\u{430}\u{439}\u{434}\u{435}\u{43d}\u{43e} \u{43d}\u{438} \u{43e}\u{434}\u{43d}\u{43e}\u{433}\u{43e} \u{431}\u{43b}\u{43e}\u{447}\u{43d}\u{43e}\u{433}\u{43e} \u{443}\u{441}\u{442}\u{440}\u{43e}\u{439}\u{441}\u{442}\u{432}\u{430}!");
        return None;
    }

    let mut label_bufs = [[0u8; 58]; 8];
    let mut label_lens = [0usize; 8];
    let real = count.min(8);

    for i in 0..real {
        let d = &disks[i];
        let name = d.name().as_bytes();
        let mut pos = 0;
        let nlen = name.len().min(16);
        label_bufs[i][pos..pos + nlen].copy_from_slice(&name[..nlen]);
        pos += nlen;
        while pos < 18 { label_bufs[i][pos] = b' '; pos += 1; }
        pos += u64_to_buf(&mut label_bufs[i][pos..], d.size_mib());
        let sfx = b" MiB  [VirtIO / ATA Disk]                  ";
        let slen = sfx.len().min(label_bufs[i].len() - pos);
        label_bufs[i][pos..pos + slen].copy_from_slice(&sfx[..slen]);
        pos += slen;
        label_lens[i] = pos;
    }

    hide_cursor();
    clear_screen();
    draw_header("  \u{42d}\u{422}\u{410}\u{41f} 1: \u{412}\u{42b}\u{411}\u{41e}\u{420} \u{426}\u{415}\u{41b}\u{415}\u{412}\u{41e}\u{413}\u{41e} \u{414}\u{418}\u{421}\u{41a}\u{410}  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "\u{412}\u{44b}\u{431}\u{435}\u{440}\u{438}\u{442}\u{435} \u{43d}\u{430}\u{43a}\u{43e}\u{43f}\u{438}\u{442}\u{435}\u{43b}\u{44c} \u{434}\u{43b}\u{44f} \u{443}\u{441}\u{442}\u{430}\u{43d}\u{43e}\u{432}\u{43a}\u{438} Wunix OS:");
    draw_centered(10, "\u{412}\u{44b}\u{431}\u{440}\u{430}\u{43d}\u{43d}\u{44b}\u{439} \u{434}\u{438}\u{441}\u{43a} \u{431}\u{443}\u{434}\u{435}\u{442} \u{43f}\u{435}\u{440}\u{435}\u{440}\u{430}\u{437}\u{43c}\u{435}\u{447}\u{435}\u{43d} (MBR + FAT32):");

    let mut items: [MenuItem; 8] = [
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
    ];
    for i in 0..real {
        items[i].label = core::str::from_utf8(&label_bufs[i][..label_lens[i]]).unwrap_or("?");
    }

    draw_separator();
    draw_footer(" \u{2191}\u{2193} \u{412}\u{44b}\u{431}\u{43e}\u{440}    Enter \u{412}\u{44b}\u{431}\u{440}\u{430}\u{442}\u{44c}    Esc \u{41d}\u{430}\u{437}\u{430}\u{434} ");

    let sel = run_menu(12, 12, &items[..real], 56)?;
    Some(&disks[sel])
}

// ─── Экран 3: Выбор файловой системы ─────────────────────────────────────────

#[derive(Copy, Clone)]
pub enum FsChoice { Fat32 }

pub fn screen_select_fs() -> Option<FsChoice> {
    hide_cursor();
    clear_screen();
    draw_header("  \u{42d}\u{422}\u{410}\u{41f} 1: \u{422}\u{418}\u{41f} \u{424}\u{410}\u{419}\u{41b}\u{41e}\u{412}\u{41e}\u{419} \u{421}\u{418}\u{421}\u{422}\u{415}\u{41c}\u{42b}  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "\u{412}\u{44b}\u{431}\u{435}\u{440}\u{438}\u{442}\u{435} \u{442}\u{438}\u{43f} \u{444}\u{430}\u{439}\u{43b}\u{43e}\u{432}\u{43e}\u{439} \u{441}\u{438}\u{441}\u{442}\u{435}\u{43c}\u{44b} \u{434}\u{43b}\u{44f} \u{440}\u{430}\u{437}\u{434}\u{435}\u{43b}\u{430}:");

    let items = [
        MenuItem { label: "1. FAT32  \u{2014} \u{448}\u{442}\u{430}\u{442}\u{43d}\u{430}\u{44f} \u{424}\u{421} Wunix (\u{43f}\u{43e}\u{434}\u{434}\u{435}\u{440}\u{436}\u{43a}\u{430} \u{447}\u{442}\u{435}\u{43d}\u{438}\u{44f}/\u{437}\u{430}\u{43f}\u{438}\u{441}\u{438})     ", disabled: false },
        MenuItem { label: "2. EXT4   \u{2014} Linux Journaling FS  (\u{437}\u{430}\u{43f}\u{43b}\u{430}\u{43d}\u{438}\u{440}\u{43e}\u{432}\u{430}\u{43d}\u{43e} \u{43d}\u{430} \u{44d}\u{442}\u{430}\u{43f} 15)  ", disabled: true  },
        MenuItem { label: "3. Btrfs  \u{2014} CoW Next-Gen FS     (\u{437}\u{430}\u{43f}\u{43b}\u{430}\u{43d}\u{438}\u{440}\u{43e}\u{432}\u{430}\u{43d}\u{43e} \u{43d}\u{430} \u{44d}\u{442}\u{430}\u{43f} 15)  ", disabled: true  },
    ];

    draw_separator();
    draw_footer(" \u{2191}\u{2193} \u{412}\u{44b}\u{431}\u{43e}\u{440}    Enter \u{41f}\u{43e}\u{434}\u{442}\u{432}\u{435}\u{440}\u{434}\u{438}\u{442}\u{44c}    Esc \u{41d}\u{430}\u{437}\u{430}\u{434} ");

    run_menu(12, 11, &items, 58)?;
    Some(FsChoice::Fat32)
}

// ─── Экран 4: Подтверждение разметки ─────────────────────────────────────────

pub fn screen_confirm_partition(disk_name: &str) -> bool {
    hide_cursor();
    clear_screen();
    draw_header("  ПОДТВЕРЖДЕНИЕ РАЗМЕТКИ  ");
    draw_logo(LOGO_ROW);

    draw_box(9, 8, 64, 6);

    let warn = "▲  ВНИМАНИЕ: ДАННЫЕ НА ДИСКЕ БУДУТ УНИЧТОЖЕНЫ  ▲";
    let wcol = 8 + (64 - 46) / 2;
    goto(10, wcol);
    wrs(BOLD); wrs(FG_YELLOW); wrs(warn); wrs(RESET);

    goto(11, 11);
    wrs(FG_WHITE);
    wrs("Накопитель ");
    wrs(FG_YELLOW); wrs(BOLD); wrs(disk_name); wrs(RESET); wrs(FG_WHITE);
    wrs(" будет размечен (MBR, FAT32 сектор 2048).");

    goto(12, 11);
    wrs(FG_RED); wrs(BOLD);
    wrs("Все текущие разделы и файлы будут безвозвратно удалены.");
    wrs(RESET);

    let items = [
        MenuItem { label: "1. Да, выполнить разметку и установку      ", disabled: false },
        MenuItem { label: "2. Нет, вернуться назад                          ", disabled: false },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    Esc Отмена ");

    match run_menu(16, 18, &items, 46) {
        Some(0) => true,
        _ => false,
    }
}

// ─── Экран 5: Настройка сети (Network Configuration) ─────────────────────────

pub fn screen_network_config() -> Option<NetConfig> {
    let mut cfg = detect_current_network();

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 3: НАСТРОЙКА СЕТИ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Настройка параметров сетевого подключения (eth0):");

    let items = [
        MenuItem { label: "1. Автоматическая настройка (DHCP)                    ", disabled: false },
        MenuItem { label: "2. Ручная настройка (Статический IP-адрес)            ", disabled: false },
    ];

    draw_separator();
    draw_footer(" ↑↓ Выбор    Enter Подтвердить    Esc Назад ");

    let mode_sel = run_menu(12, 11, &items, 58)?;
    if mode_sel == 0 {
        // DHCP режим
        cfg.is_dhcp = true;
        screen_dhcp_summary(&cfg);
    } else {
        // Статический режим
        cfg.is_dhcp = false;
        if !screen_static_network_form(&mut cfg) {
            return None;
        }
    }

    // Проверка соединения через Ping
    screen_ping_verification(&cfg);

    Some(cfg)
}

fn screen_dhcp_summary(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  СЕТЬ: АВТОКОНФИГУРАЦИЯ (DHCP)  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Параметры сети, полученные от DHCP-сервера:");

    draw_box(11, 14, 52, 6);
    goto(12, 18); wrs(FG_GRAY); wrs("IP-адрес      : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.ip_str()); wrs(RESET);
    goto(13, 18); wrs(FG_GRAY); wrs("Маска подсети : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.netmask_str()); wrs(RESET);
    goto(14, 18); wrs(FG_GRAY); wrs("Шлюз (Gateway): "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.gateway_str()); wrs(RESET);
    goto(15, 18); wrs(FG_GRAY); wrs("DNS-сервер    : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.dns_str()); wrs(RESET);

    draw_separator();
    draw_footer(" Enter Продолжить ");
    wait_key();
}

fn screen_static_network_form(cfg: &mut NetConfig) -> bool {
    hide_cursor();
    clear_screen();
    draw_header("  СТАТИЧЕСКАЯ НАСТРОЙКА СЕТИ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Введите сетевые параметры вручную:");

    let label_col = 30usize;
    let field_col = 32usize;
    let field_w   = 24usize;

    draw_label(12, "IP-адрес", label_col);
    draw_label(14, "Маска подсети", label_col);
    draw_label(16, "Основной шлюз", label_col);
    draw_label(18, "DNS-сервер", label_col);

    draw_input_field(12, field_col, field_w, &cfg.ip, cfg.ip_len);
    draw_input_field(14, field_col, field_w, &cfg.netmask, cfg.netmask_len);
    draw_input_field(16, field_col, field_w, &cfg.gateway, cfg.gateway_len);
    draw_input_field(18, field_col, field_w, &cfg.dns, cfg.dns_len);

    draw_separator();
    draw_footer(" Enter Следующее поле    Esc Отмена ");

    let mut buf = [0u8; 32];
    buf[..cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]);
    let l1 = input_text(12, field_col, &mut buf, field_w);
    if l1 > 0 { cfg.ip[..l1].copy_from_slice(&buf[..l1]); cfg.ip_len = l1; }

    buf = [0; 32];
    buf[..cfg.netmask_len].copy_from_slice(&cfg.netmask[..cfg.netmask_len]);
    let l2 = input_text(14, field_col, &mut buf, field_w);
    if l2 > 0 { cfg.netmask[..l2].copy_from_slice(&buf[..l2]); cfg.netmask_len = l2; }

    buf = [0; 32];
    buf[..cfg.gateway_len].copy_from_slice(&cfg.gateway[..cfg.gateway_len]);
    let l3 = input_text(16, field_col, &mut buf, field_w);
    if l3 > 0 { cfg.gateway[..l3].copy_from_slice(&buf[..l3]); cfg.gateway_len = l3; }

    buf = [0; 32];
    buf[..cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]);
    let l4 = input_text(18, field_col, &mut buf, field_w);
    if l4 > 0 { cfg.dns[..l4].copy_from_slice(&buf[..l4]); cfg.dns_len = l4; }

    true
}

fn screen_ping_verification(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 3: ПРОВЕРКА СЕТЕВОГО ПОДКЛЮЧЕНИЯ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Желаете проверить сетевое подключение перед продолжением?");

    let items = [
        MenuItem { label: "1. Проверить сетевое подключение (пинг шлюза)          ", disabled: false },
        MenuItem { label: "2. Пропустить проверку и перейти к созданию учётной записи ", disabled: false },
    ];

    draw_separator();
    draw_footer(" ↑↓ Выбор    Enter Подтвердить ");

    let choice = run_menu(11, 10, &items, 60);
    if choice == Some(0) {
        let gw_ip = parse_ipv4(cfg.gateway_str()).unwrap_or([10, 0, 2, 2]);

        goto(14, 12);
        wrs(FG_YELLOW);
        wrs("Проверка соединения со шлюзом ");
        wrs(cfg.gateway_str());
        wrs("...");
        wrs(RESET);

        let mut success_count = 0;
        let mut min_rtt = 9999u64;

        ping_target_probes(gw_ip, 2, |seq, res| {
            goto(14 + seq, 12);
            wr(b"\x1b[2K");
            goto(14 + seq, 12);
            match res {
                Some(rtt) => {
                    success_count += 1;
                    if rtt < min_rtt { min_rtt = rtt; }
                    wrs(FG_GREEN); wr(b" ["); write_u64(seq as u64); wr(b"/2] \xe2\x9c\x93  "); wrs(RESET);
                    wrs("Ответ от "); wrs(cfg.gateway_str());
                    wrs(": icmp_seq="); write_u64(seq as u64);
                    wrs(" время="); write_u64(rtt); wrs(" мс");
                }
                None => {
                    wrs(FG_RED); wr(b" ["); write_u64(seq as u64); wr(b"/2] \xe2\x9c\x97  "); wrs(RESET);
                    wrs("превышен интервал ожидания (icmp_seq="); write_u64(seq as u64); wrs(")");
                }
            }
        });

        goto(18, 12);
        if success_count > 0 {
            draw_status(18, 12, true, "Сеть работает корректно! Соединение подтверждено.");
            goto(19, 12);
            wrs(FG_GREEN);
            wrs("Шлюз "); wrs(cfg.gateway_str());
            wrs(" доступен (задержка: ");
            write_u64(min_rtt);
            wrs(" мс).");
            wrs(RESET);
        } else {
            draw_status(18, 12, false, "Шлюз не ответил на ICMP эхо-запрос.");
            goto(19, 12);
            wrs(FG_YELLOW);
            wrs("Параметры сохранены. Установка продолжается.");
            wrs(RESET);
        }

        draw_footer(" Enter Продолжить установку ");
        wait_key();
    }
}

// ─── Экран 6: Создание пользователя (User & Sudo) ────────────────────────────

pub struct UserConfig {
    pub username: [u8; 64],
    pub username_len: usize,
    pub password: [u8; 128],
    pub password_len: usize,
}

pub fn screen_create_user() -> Option<UserConfig> {
    let mut cfg = UserConfig {
        username: [0; 64],
        username_len: 0,
        password: [0; 128],
        password_len: 0,
    };

    let mut error_msg: Option<&str> = None;

    loop {
        hide_cursor();
        clear_screen();
        draw_header("  ЭТАП 4: СОЗДАНИЕ ПОЛЬЗОВАТЕЛЯ (SUDO)  ");
        draw_logo(LOGO_ROW);

        draw_centered(9, "Создание первого администратора (wheel / sudo):");

        let label_col = 26usize;
        let field_col = 28usize;
        let field_w   = 40usize;

        draw_label(11, "Имя пользователя", label_col);
        draw_label(13, "Пароль", label_col);
        draw_label(15, "Повтор пароля", label_col);

        draw_input_field(11, field_col, field_w, &cfg.username, cfg.username_len);
        draw_password_field(13, field_col, field_w, cfg.password_len);
        draw_password_field(15, field_col, field_w, 0);

        goto(17, 14);
        wrs(FG_GRAY); wrs(DIM);
        wrs("Пароль root будет таким же, как у первого администратора.");
        wrs(RESET);

        if let Some(err) = error_msg {
            draw_info(18, err, FG_RED);
        }

        draw_separator();
        draw_footer(" Enter Следующее поле    Esc Назад ");

        cfg.username = [0; 64];
        cfg.username_len = input_text(11, field_col, &mut cfg.username, field_w);
        if cfg.username_len == 0 { return None; }

        cfg.password = [0; 128];
        cfg.password_len = input_password(13, field_col, &mut cfg.password, field_w);

        let mut pass2 = [0u8; 128];
        let pass2_len = input_password(15, field_col, &mut pass2, field_w);

        if cfg.password[..cfg.password_len] != pass2[..pass2_len] {
            error_msg = Some("Пароли не совпадают! Повторите ввод.");
            cfg.password_len = 0;
            continue;
        }

        return Some(cfg);
    }
}

// ─── Экран 7: Имя хоста (Hostname) ───────────────────────────────────────────

pub fn screen_hostname() -> Option<([u8; 64], usize)> {
    let mut buf = [0u8; 64];
    let default = b"wunix";
    buf[..default.len()].copy_from_slice(default);

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 5: ИМЯ КОМПЬЮТЕРА (HOSTNAME)  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Введите сетевое имя компьютера (hostname):");

    let label_col = 26usize;
    let field_col = 28usize;
    let field_w   = 40usize;

    draw_label(12, "Hostname", label_col);
    draw_input_field(12, field_col, field_w, &buf, default.len());

    goto(15, 12);
    wrs(FG_GRAY); wrs(DIM);
    wrs("Используется для идентификации в сети и в wsh (например, user@wunix:~$):");
    wrs(RESET);

    draw_separator();
    draw_footer(" Enter Подтвердить    Esc Назад ");

    let len = input_text(12, field_col, &mut buf, field_w);
    let final_len = if len == 0 {
        buf[..default.len()].copy_from_slice(default);
        default.len()
    } else {
        len
    };
    Some((buf, final_len))
}

// ─── Экран 8: Прогресс установки ─────────────────────────────────────────────

pub fn begin_progress_screen() {
    hide_cursor();
    clear_screen();
    draw_header("  УСТАНОВКА WUNIX BASE  ");
    draw_logo(LOGO_ROW);

    draw_centered(10, "Производится установка системы на диск, подождите...");
    draw_box(12, 8, 64, 7);

    draw_separator();
    draw_footer(" Не перезагружайте машину до завершения ");
}

pub fn update_progress(percent: u8, stage: &str, files_done: usize, files_total: usize) {
    let box_right = 8 + 64 - 1; // 71

    // Строка 14: Текущий этап установки
    goto(14, 11);
    wrs(FG_WHITE);
    wrs("Этап: ");
    wrs(stage);
    let st_chars = crate::tui::char_count(stage);
    let used_stage = 6 + st_chars;
    let max_inner = 59usize;
    let pad_stage = max_inner.saturating_sub(used_stage);
    for _ in 0..pad_stage { wr(b" "); }
    goto(14, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);

    // Строка 16: Индикатор прогресса
    draw_progress(16, 11, 44, percent);
    goto(16, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);

    // Строка 17: Количество скопированных файлов
    goto(17, 11);
    wrs(FG_GRAY);
    wrs("Компонентов: ");
    write_u64(files_done as u64);
    wr(b" / ");
    write_u64(files_total as u64);
    let mut done_digits = 1;
    let mut d = files_done;
    while d >= 10 { done_digits += 1; d /= 10; }
    let mut tot_digits = 1;
    let mut t = files_total;
    while t >= 10 { tot_digits += 1; t /= 10; }
    let used_comp = 13 + done_digits + 3 + tot_digits;
    let pad_comp = max_inner.saturating_sub(used_comp);
    for _ in 0..pad_comp { wr(b" "); }
    goto(17, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);
}

// ─── Экран 9: Завершение установки ───────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FinishChoice { Reboot, Shutdown, Shell }

pub fn screen_finish(disk: &str, username: &str, hostname: &str, ip: &str) -> FinishChoice {
    hide_cursor();
    clear_screen();
    draw_header("  УСТАНОВКА УСПЕШНО ЗАВЕРШЕНА  ");
    draw_logo(LOGO_ROW);

    draw_info(9, "✓  Система Wunix OS успешно установлена на диск!", FG_GREEN);

    goto(10, 14); wrs(FG_GRAY); wrs("Целевой накопитель   : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(disk); wrs(RESET);
    goto(11, 14); wrs(FG_GRAY); wrs("Пользователь (sudo)  : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(username); wrs(RESET);
    goto(12, 14); wrs(FG_GRAY); wrs("Имя хоста (Hostname) : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(hostname); wrs(RESET);
    goto(13, 14); wrs(FG_GRAY); wrs("Сетевой IP-адрес     : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(ip); wrs(RESET);

    draw_centered(15, "Благодарим вас за тестирование Wunix OS!");
    draw_centered(16, "Извлеките установочный ISO и запустите систему с жесткого диска.");

    let items = [
        MenuItem { label: "1. Перезагрузить систему                           ", disabled: false },
        MenuItem { label: "2. Выключить машину (для загрузки с диска)         ", disabled: false },
        MenuItem { label: "3. Выйти в командную оболочку (wsh)                ", disabled: false },
    ];

    draw_separator();
    draw_footer(" ↑↓ Выбор    Enter Подтвердить ");

    match run_menu_compact(17, 14, &items, 52) {
        Some(0) => FinishChoice::Reboot,
        Some(1) => FinishChoice::Shutdown,
        _ => FinishChoice::Shell,
    }
}

// ─── Экран ошибки ────────────────────────────────────────────────────────────

pub fn screen_error(msg: &str) {
    hide_cursor();
    clear_screen();
    draw_header("  \u{41e}\u{428}\u{418}\u{411}\u{41a}\u{410}  ");
    draw_logo(LOGO_ROW);

    draw_box(13, 8, 64, 5);
    goto(15, 12);
    wrs(FG_RED); wrs(BOLD);
    wrs(msg);
    wrs(RESET);

    draw_info(19, "\u{41d}\u{430}\u{436}\u{43c}\u{438}\u{442}\u{435} \u{43b}\u{44e}\u{431}\u{443}\u{44e} \u{43a}\u{43b}\u{430}\u{432}\u{438}\u{448}\u{443}...", FG_GRAY);
    draw_separator();
    draw_footer(" \u{41d}\u{430}\u{436}\u{43c}\u{438}\u{442}\u{435} \u{43b}\u{44e}\u{431}\u{443}\u{44e} \u{43a}\u{43b}\u{430}\u{432}\u{438}\u{448}\u{443} ");
    wait_key();
}
