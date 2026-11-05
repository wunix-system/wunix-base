// Выбор сетевого адаптера, настройка DHCP/Static и ICMP-тестирование ya.ru и google.com.

use crate::net::{
    detect_adapter_network, run_ping_four_packets, NetAdapterInfo, NetConfig,
};
use crate::tui::{
    clear_screen, draw_box, draw_centered, draw_footer, draw_header, draw_input_field,
    draw_label, draw_logo, draw_separator, draw_status, goto, hide_cursor, input_text,
    run_menu, wait_key, write_u64, wr, wrs, MenuItem, BOLD, FG_GRAY,
    FG_GREEN, FG_RED, FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
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

        draw_box(10, 8, 64, 7);
        goto(12, 12);
        wrs(FG_YELLOW); wrs(BOLD);
        wrs("В системе не обнаружено ни одного сетевого адаптера!");
        wrs(RESET);

        goto(14, 12);
        wrs(FG_WHITE);
        wrs("Установка Wunix OS будет продолжена в автономном режиме");
        goto(15, 12);
        wrs("без настройки сетевого интерфейса.");

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
    draw_footer(" ↑↓ Выбор    Enter Выбрать адаптер    Esc Назад ");

    let sel = run_menu(12, 10, &items[..real], 56)?;
    Some(Some(&adapters[sel]))
}

/// Настройка выбранного сетевого адаптера (DHCP / Static)
pub fn screen_network_config(adapter: &NetAdapterInfo) -> Option<NetConfig> {
    let mut cfg = detect_adapter_network(adapter.name_str());

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: НАСТРОЙКА СЕТИ  ");
    draw_logo(LOGO_ROW);

    goto(9, 10);
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
    draw_footer(" ↑↓ Выбор    Enter Подтвердить    Esc Назад ");

    let mode_sel = run_menu(12, 8, &items, 64)?;
    if mode_sel == 0 {
        cfg.is_dhcp = true;
        screen_dhcp_summary(&cfg);
    } else {
        cfg.is_dhcp = false;
        if !screen_static_network_form(&mut cfg) {
            return None;
        }
    }

    // Проверка соединения через пинг ya.ru и google.com
    screen_ping_four_packets(&cfg);

    Some(cfg)
}

fn screen_dhcp_summary(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ПАРАМЕТРЫ СЕТИ: DHCP  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Реальные параметры сети, полученные от DHCP-сервера:");

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

/// Проверка сети: 4 пакета — 2 пакета на ya.ru и 2 на google.com
fn screen_ping_four_packets(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: ПРОВЕРКА СВЯЗИ С ИНТЕРНЕТОМ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Желаете протестировать подключение к сети Интернет (4 пакета)?");

    let items = [
        MenuItem {
            label: "1. Проверить сеть (2 пакета ya.ru + 2 пакета google.com)       ",
            disabled: false,
        },
        MenuItem {
            label: "2. Пропустить проверку и перейти к созданию учётной записи     ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" ↑↓ Выбор    Enter Подтвердить ");

    let choice = run_menu(11, 8, &items, 64);
    if choice != Some(0) {
        return;
    }

    goto(13, 8);
    wrs(FG_YELLOW);
    wrs("Выполняется отправка 4 пакетов (DNS-резолв + ICMP Echo)...");
    wrs(RESET);

    let mut successful = 0usize;

    let (_ya, _goog) = run_ping_four_packets(cfg, |host, seq, total, res| {
        let row = 13 + seq;
        goto(row, 8);
        wr(b"\x1b[2K");
        goto(row, 8);
        match res {
            Some(rtt) => {
                successful += 1;
                wrs(FG_GREEN); wr(b" ["); write_u64(seq as u64); wr(b"/"); write_u64(total as u64); wr(b"] \xe2\x9c\x93 "); wrs(RESET);
                wrs("Ответ от "); wrs(host); wrs(": время="); write_u64(rtt); wrs(" мс");
            }
            None => {
                wrs(FG_RED); wr(b" ["); write_u64(seq as u64); wr(b"/"); write_u64(total as u64); wr(b"] \xe2\x9c\x97 "); wrs(RESET);
                wrs("Превышен интервал ожидания от "); wrs(host);
            }
        }
    });

    goto(18, 8);
    if successful >= 2 {
        draw_status(18, 8, true, "Подключение к сети Интернет подтверждено (пакеты доставлены)!");
    } else if successful >= 1 {
        draw_status(18, 8, true, "Частичный ответ (часть пакетов доставлена). Связь установлена.");
    } else {
        draw_status(18, 8, false, "Внешние сайты не ответили. Настройки сохранены для работы в LAN.");
    }

    draw_separator();
    draw_footer(" Enter Продолжить установку ");
    wait_key();
}
