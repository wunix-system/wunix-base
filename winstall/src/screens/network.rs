// Выбор сетевого адаптера, настройка DHCP/Static и фоновая проверка подключения.

use crate::net::{
    detect_adapter_network, run_ping_four_packets, NetAdapterInfo, NetConfig,
};
use crate::tui::{
    center_col, clear_screen, draw_box, draw_centered, draw_footer, draw_header,
    draw_input_field, draw_label, draw_logo, draw_separator, draw_status, goto,
    hide_cursor, input_text_nav, run_menu, wait_key, write_u64, wr, wrs, InputExit,
    MenuItem, BOLD, FG_GRAY, FG_GREEN, FG_RED, FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
};

/// Экран выбора сетевого адаптера. Если адаптеров нет — информирует об автономном режиме.
pub fn screen_select_adapter<'a>(
    adapters: &'a [NetAdapterInfo],
    count: usize,
) -> Option<Option<&'a NetAdapterInfo>> {
    if count == 0 {
        hide_cursor();
        clear_screen();
        draw_header("  СЕТЕВЫЕ АДАПТЕРЫ НЕ НАЙДЕНЫ  ");
        draw_logo(LOGO_ROW);

        let box_w = 68;
        let col = center_col(box_w);
        draw_box(10, col, box_w, 7);

        goto(12, col + 4);
        wrs(FG_YELLOW); wrs(BOLD);
        wrs("В системе не обнаружено доступных сетевых адаптеров!");
        wrs(RESET);

        goto(14, col + 4);
        wrs(FG_WHITE);
        wrs("Установка Wunix OS будет продолжена в автономном режиме");
        goto(15, col + 4);
        wrs("без предварительной конфигурации сетевого интерфейса.");

        draw_separator();
        draw_footer(" Enter Продолжить установку ");
        wait_key();
        return Some(None);
    }

    let mut label_bufs = [[0u8; 64]; 4];
    let mut label_lens = [0usize; 4];
    let real = count.min(4);

    for i in 0..real {
        let a = &adapters[i];
        let name = a.name_str().as_bytes();
        let mut pos = 0;
        let nlen = name.len().min(8);
        label_bufs[i][pos..pos + nlen].copy_from_slice(&name[..nlen]);
        pos += nlen;
        while pos < 10 { label_bufs[i][pos] = b' '; pos += 1; }

        let mac = a.mac_str().as_bytes();
        let mlen = mac.len().min(18);
        label_bufs[i][pos..pos + mlen].copy_from_slice(&mac[..mlen]);
        pos += mlen;
        while pos < 30 { label_bufs[i][pos] = b' '; pos += 1; }

        let sfx = if a.is_up { b" [Link: UP]   " as &[u8] } else { b" [Link: DOWN] " as &[u8] };
        label_bufs[i][pos..pos + sfx.len()].copy_from_slice(sfx);
        pos += sfx.len();
        label_lens[i] = pos;
    }

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: ВЫБОР СЕТЕВОГО АДАПТЕРА  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Обнаружены следующие сетевые интерфейсы:");

    let mut items: [MenuItem; 4] = [
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
    ];
    for i in 0..real {
        items[i].label = core::str::from_utf8(&label_bufs[i][..label_lens[i]]).unwrap_or("?");
    }

    draw_separator();
    draw_footer(" Enter Выбрать адаптер    ↑↓ Навигация    Esc Назад ");

    let menu_w = 56;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items[..real], menu_w)?;
    Some(Some(&adapters[sel]))
}

/// Настройка выбранного сетевого адаптера (DHCP / Static)
pub fn screen_network_config(adapter: &NetAdapterInfo) -> Option<NetConfig> {
    let mut cfg = detect_adapter_network(adapter.name_str());

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: НАСТРОЙКА СЕТИ  ");
    draw_logo(LOGO_ROW);

    let menu_w = 64;
    let col = center_col(menu_w);

    goto(9, col);
    wrs(FG_WHITE);
    wrs("Настройка параметров подключения для интерфейса ");
    wrs(FG_YELLOW); wrs(BOLD); wrs(adapter.name_str()); wrs(RESET); wrs(FG_WHITE);
    wrs(":");

    let items = [
        MenuItem {
            label: "1. Автоматическая настройка (DHCP — получение от роутера) ",
            disabled: false,
        },
        MenuItem {
            label: "2. Ручная настройка (Статический IP, маска, шлюз, DNS)     ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    ↑↓ Навигация    Esc Назад ");

    let mode_sel = run_menu(12, col, &items, menu_w)?;
    if mode_sel == 0 {
        cfg.is_dhcp = true;
        screen_dhcp_summary(&cfg);
    } else {
        cfg.is_dhcp = false;
        if !screen_static_network_form(&mut cfg) {
            return None;
        }
    }

    // Проверка сетевого соединения
    screen_network_check(&cfg);

    Some(cfg)
}

fn screen_dhcp_summary(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ПАРАМЕТРЫ СЕТИ: DHCP  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Реальные параметры сети, полученные от DHCP-сервера:");

    let box_w = 60;
    let col = center_col(box_w);

    draw_box(11, col, box_w, 6);
    goto(12, col + 4); wrs(FG_GRAY); wrs("IP-адрес      : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.ip_str()); wrs(RESET);
    goto(13, col + 4); wrs(FG_GRAY); wrs("Маска подсети : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.netmask_str()); wrs(RESET);
    goto(14, col + 4); wrs(FG_GRAY); wrs("Шлюз (Gateway): "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.gateway_str()); wrs(RESET);
    goto(15, col + 4); wrs(FG_GRAY); wrs("DNS-сервер    : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.dns_str()); wrs(RESET);

    draw_separator();
    draw_footer(" Enter Продолжить ");
    wait_key();
}

fn screen_static_network_form(cfg: &mut NetConfig) -> bool {
    let mut cur_field: usize = 0;
    let mut ip_buf = [0u8; 32];
    ip_buf[..cfg.ip_len].copy_from_slice(&cfg.ip[..cfg.ip_len]);
    let mut nm_buf = [0u8; 32];
    nm_buf[..cfg.netmask_len].copy_from_slice(&cfg.netmask[..cfg.netmask_len]);
    let mut gw_buf = [0u8; 32];
    gw_buf[..cfg.gateway_len].copy_from_slice(&cfg.gateway[..cfg.gateway_len]);
    let mut dns_buf = [0u8; 32];
    dns_buf[..cfg.dns_len].copy_from_slice(&cfg.dns[..cfg.dns_len]);

    loop {
        hide_cursor();
        clear_screen();
        draw_header("  СТАТИЧЕСКАЯ НАСТРОЙКА СЕТИ  ");
        draw_logo(LOGO_ROW);

        draw_centered(9, "Введите сетевые параметры вручную:");

        let form_w = 54;
        let base_col = center_col(form_w);
        let label_col = base_col + 18;
        let field_col = base_col + 20;
        let field_w   = 28usize;

        draw_label(12, "IP-адрес", label_col);
        draw_label(14, "Маска подсети", label_col);
        draw_label(16, "Основной шлюз", label_col);
        draw_label(18, "DNS-сервер", label_col);

        let l_ip = ip_buf.iter().position(|&b| b == 0).unwrap_or(ip_buf.len());
        let l_nm = nm_buf.iter().position(|&b| b == 0).unwrap_or(nm_buf.len());
        let l_gw = gw_buf.iter().position(|&b| b == 0).unwrap_or(gw_buf.len());
        let l_dns = dns_buf.iter().position(|&b| b == 0).unwrap_or(dns_buf.len());

        draw_input_field(12, field_col, field_w, &ip_buf, l_ip);
        draw_input_field(14, field_col, field_w, &nm_buf, l_nm);
        draw_input_field(16, field_col, field_w, &gw_buf, l_gw);
        draw_input_field(18, field_col, field_w, &dns_buf, l_dns);

        draw_separator();
        draw_footer(" Enter Подтвердить    Tab/↑↓ Переключение полей    Esc Отмена ");

        match cur_field {
            0 => {
                let (len, exit) = input_text_nav(12, field_col, &mut ip_buf, field_w);
                cfg.ip[..len].copy_from_slice(&ip_buf[..len]);
                cfg.ip_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 1,
                    InputExit::Prev => cur_field = 3,
                    InputExit::Cancel => return false,
                }
            }
            1 => {
                let (len, exit) = input_text_nav(14, field_col, &mut nm_buf, field_w);
                cfg.netmask[..len].copy_from_slice(&nm_buf[..len]);
                cfg.netmask_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 2,
                    InputExit::Prev => cur_field = 0,
                    InputExit::Cancel => return false,
                }
            }
            2 => {
                let (len, exit) = input_text_nav(16, field_col, &mut gw_buf, field_w);
                cfg.gateway[..len].copy_from_slice(&gw_buf[..len]);
                cfg.gateway_len = len;
                match exit {
                    InputExit::Next | InputExit::Submit => cur_field = 3,
                    InputExit::Prev => cur_field = 1,
                    InputExit::Cancel => return false,
                }
            }
            3 => {
                let (len, exit) = input_text_nav(18, field_col, &mut dns_buf, field_w);
                cfg.dns[..len].copy_from_slice(&dns_buf[..len]);
                cfg.dns_len = len;
                match exit {
                    InputExit::Submit => return true,
                    InputExit::Next => cur_field = 0,
                    InputExit::Prev => cur_field = 2,
                    InputExit::Cancel => return false,
                }
            }
            _ => cur_field = 0,
        }
    }
}

/// Проверка подключения к сети с отображением статусов в реальном времени
pub fn screen_network_check(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: ПРОВЕРКА ПОДКЛЮЧЕНИЯ К СЕТИ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Желаете выполнить быструю проверку подключения к сети?");

    let items = [
        MenuItem {
            label: "1. Выполнить проверку связи (Ping шлюза, DNS и внешних узлов)",
            disabled: false,
        },
        MenuItem {
            label: "2. Пропустить проверку и перейти к созданию учётной записи     ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Выбрать    Tab/↑↓ Переключение ");

    let menu_w = 68;
    let col = center_col(menu_w);
    let choice = run_menu(11, col, &items, menu_w);
    if choice != Some(0) {
        return;
    }

    // Выполняем проверку
    clear_screen();
    draw_header("  ЭТАП 4: ПРОВЕРКА ПОДКЛЮЧЕНИЯ К СЕТИ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Диагностика сетевого стека и проверка связи:");

    let box_w = 64;
    let bcol = center_col(box_w);

    draw_box(11, bcol, box_w, 8);
    goto(12, bcol + 4); wrs(FG_GRAY); wrs("Интерфейс : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.adapter_str()); wrs(RESET);
    wrs(FG_GREEN); wrs(" [UP]"); wrs(RESET);

    goto(13, bcol + 4); wrs(FG_GRAY); wrs("IP-адрес  : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(cfg.ip_str()); wrs(RESET);
    if cfg.is_dhcp { wrs(FG_GRAY); wrs(" (DHCP)"); wrs(RESET); }

    goto(14, bcol + 4); wrs(FG_GRAY); wrs("Шлюз      : "); wrs(RESET); wrs(FG_WHITE); wrs(cfg.gateway_str()); wrs(RESET);
    wrs(FG_YELLOW); wrs(" (проверка...)"); wrs(RESET);

    goto(15, bcol + 4); wrs(FG_GRAY); wrs("DNS       : "); wrs(RESET); wrs(FG_WHITE); wrs(cfg.dns_str()); wrs(RESET);
    wrs(FG_YELLOW); wrs(" (проверка...)"); wrs(RESET);

    goto(16, bcol + 4); wrs(FG_GRAY); wrs("Интернет  : "); wrs(RESET); wrs(FG_YELLOW); wrs("проверка..."); wrs(RESET);

    draw_separator();
    draw_footer(" Выполняется проверка связи... ");

    let (res_gw, res_ext) = run_ping_four_packets(cfg, |_host, _seq, _total, _res| {});

    // Обновляем статус шлюза
    goto(14, bcol + 4); wr(b"\x1b[2K");
    goto(14, bcol + 4); wrs(FG_GRAY); wrs("Шлюз      : "); wrs(RESET); wrs(FG_WHITE); wrs(cfg.gateway_str()); wrs(RESET);
    if let Some(rtt) = res_gw.rtt1 {
        wrs(FG_GREEN); wrs(" [OK] ("); write_u64(rtt); wrs(" мс)"); wrs(RESET);
    } else {
        wrs(FG_RED); wrs(" [Нет ответа]"); wrs(RESET);
    }

    // Обновляем статус DNS
    goto(15, bcol + 4); wr(b"\x1b[2K");
    goto(15, bcol + 4); wrs(FG_GRAY); wrs("DNS       : "); wrs(RESET); wrs(FG_WHITE); wrs(cfg.dns_str()); wrs(RESET);
    if let Some(rtt) = res_gw.rtt2 {
        wrs(FG_GREEN); wrs(" [OK] ("); write_u64(rtt); wrs(" мс)"); wrs(RESET);
    } else if res_ext.dns_resolved {
        wrs(FG_GREEN); wrs(" [OK]"); wrs(RESET);
    } else {
        wrs(FG_GRAY); wrs(" [Локальный]"); wrs(RESET);
    }

    // Обновляем статус Интернета
    goto(16, bcol + 4); wr(b"\x1b[2K");
    goto(16, bcol + 4); wrs(FG_GRAY); wrs("Интернет  : "); wrs(RESET);
    if res_ext.dns_resolved || res_ext.rtt1.is_some() || res_ext.rtt2.is_some() {
        wrs(FG_GREEN); wrs(BOLD); wrs("[OK] Доступен"); wrs(RESET);
    } else if res_gw.rtt1.is_some() {
        wrs(FG_YELLOW); wrs("[Локальная сеть]"); wrs(RESET);
    } else {
        wrs(FG_RED); wrs("[Автономный режим]"); wrs(RESET);
    }

    let status_w = 54;
    let scol = center_col(status_w);
    if res_gw.rtt1.is_some() || res_ext.dns_resolved {
        draw_status(20, scol, true, "Сетевое подключение проверено и успешно работает");
    } else {
        draw_status(20, scol, false, "Связь со шлюзом отсутствует (автономный режим)");
    }

    draw_separator();
    draw_footer(" Enter Продолжить установку ");
    wait_key();
}
