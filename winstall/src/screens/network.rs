// Выбор сетевого адаптера, настройка DHCP/Static и фоновая проверка подключения.

use crate::net::{
    detect_adapter_network, run_ping_four_packets, NetAdapterInfo, NetConfig,
};
use crate::tui::{
    center_col, clear_screen, draw_box, draw_centered, draw_footer, draw_header,
    draw_input_field, draw_label, draw_logo, draw_separator, draw_status, goto,
    hide_cursor, input_text, run_menu, wait_key, wr, wrs, MenuItem, BOLD, FG_GRAY,
    FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
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

    // Фоновая проверка сети без вывода деталей запросов
    screen_network_check_quiet(&cfg);

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

/// Фоновая проверка подключения к сети:
/// Запросы выполняются без вывода логов, выдается итоговый статус (ОК / Нестабильно / Плохо)
fn screen_network_check_quiet(cfg: &NetConfig) {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 4: ПРОВЕРКА ПОДКЛЮЧЕНИЯ К СЕТИ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Желаете выполнить быструю проверку подключения к сети?");

    let items = [
        MenuItem {
            label: "1. Выполнить проверку связи в фоновом режиме                 ",
            disabled: false,
        },
        MenuItem {
            label: "2. Пропустить проверку и перейти к созданию учётной записи     ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    ↑↓ Навигация ");

    let menu_w = 64;
    let col = center_col(menu_w);
    let choice = run_menu(11, col, &items, menu_w);
    if choice != Some(0) {
        return;
    }

    // Очищаем область и выводим ненавязчивый индикатор ожидания
    goto(11, 1); wr(b"\x1b[2K");
    goto(12, 1); wr(b"\x1b[2K");
    goto(13, 1); wr(b"\x1b[2K");
    goto(14, 1); wr(b"\x1b[2K");

    draw_centered(12, "Выполняется быстрая фоновая проверка сетевого соединения...");
    draw_centered(13, "Пожалуйста, подождите несколько секунд...");

    let mut successful = 0usize;
    let _ = run_ping_four_packets(cfg, |_host, _seq, _total, res| {
        if res.is_some() {
            successful += 1;
        }
    });

    goto(12, 1); wr(b"\x1b[2K");
    goto(13, 1); wr(b"\x1b[2K");

    let status_w = 50;
    let scol = center_col(status_w);

    if successful >= 3 {
        draw_status(12, scol, true, "Статус сети: Отлично (ОК)");
        draw_centered(14, "Сетевое подключение настроено и работает стабильно.");
    } else if successful >= 1 {
        draw_status(12, scol, true, "Статус сети: Нестабильно");
        draw_centered(14, "Связь обнаружена, но возможны задержки или потери пакетов.");
    } else {
        draw_status(12, scol, false, "Статус сети: Плохо (Нет связи)");
        draw_centered(14, "Сетевые параметры сохранены для работы в локальной сети.");
    }

    draw_separator();
    draw_footer(" Enter Продолжить установку ");
    wait_key();
}
