// Настройка даты, времени и выбор часового пояса (по умолчанию Europe/Moscow — МСК).

use libwunix::time::{clock_realtime, DateTime};
use crate::tui::{
    clear_screen, draw_centered, draw_footer, draw_header, draw_input_field,
    draw_label, draw_logo, draw_separator, goto, hide_cursor,
    run_menu, MenuItem, BOLD, FG_YELLOW, LOGO_ROW, RESET, wrs,
};

pub struct TimezoneChoice {
    pub name: &'static str,
    pub display: &'static str,
}

pub const TIMEZONES: &[TimezoneChoice] = &[
    TimezoneChoice {
        name: "Europe/Moscow",
        display: "1. Europe/Moscow     (MSK, UTC+3)  [По умолчанию: Московское время]",
    },
    TimezoneChoice {
        name: "Europe/Kaliningrad",
        display: "2. Europe/Kaliningrad (EET, UTC+2)  [Калининградское время]",
    },
    TimezoneChoice {
        name: "Europe/Samara",
        display: "3. Europe/Samara      (SAMT, UTC+4) [Самарское время]",
    },
    TimezoneChoice {
        name: "Asia/Yekaterinburg",
        display: "4. Asia/Yekaterinburg (YEKT, UTC+5) [Екатеринбургское время]",
    },
    TimezoneChoice {
        name: "Asia/Novosibirsk",
        display: "5. Asia/Novosibirsk   (+07, UTC+7)  [Новосибирское время]",
    },
    TimezoneChoice {
        name: "Asia/Vladivostok",
        display: "6. Asia/Vladivostok   (+10, UTC+10) [Владивостокское время]",
    },
    TimezoneChoice {
        name: "UTC",
        display: "7. UTC                (UTC, +0)     [Всемирное координированное время]",
    },
    TimezoneChoice {
        name: "Europe/London",
        display: "8. Europe/London      (GMT, +0)     [Лондон / Западная Европа]",
    },
    TimezoneChoice {
        name: "Europe/Berlin",
        display: "9. Europe/Berlin      (CET, +1)     [Берлин / Центральная Европа]",
    },
];

pub fn screen_datetime_and_timezone() -> Option<&'static str> {
    let now = clock_realtime();
    // По умолчанию московское время (MSK = UTC+3 = 10800 сек)
    let msk_offset = 10800i32;
    let dt = DateTime::from_epoch(now.tv_sec, now.tv_nsec as u32, msk_offset);

    let mut date_buf = [0u8; 16];
    let mut time_buf = [0u8; 16];

    // Форматируем дату YYYY-MM-DD
    let mut dpos = 0;
    let y = dt.year as u32;
    date_buf[dpos] = b'0' + ((y / 1000) % 10) as u8; dpos += 1;
    date_buf[dpos] = b'0' + ((y / 100) % 10) as u8; dpos += 1;
    date_buf[dpos] = b'0' + ((y / 10) % 10) as u8; dpos += 1;
    date_buf[dpos] = b'0' + (y % 10) as u8; dpos += 1;
    date_buf[dpos] = b'-'; dpos += 1;
    let m = dt.month as u32;
    date_buf[dpos] = b'0' + ((m / 10) % 10) as u8; dpos += 1;
    date_buf[dpos] = b'0' + (m % 10) as u8; dpos += 1;
    date_buf[dpos] = b'-'; dpos += 1;
    let d = dt.day as u32;
    date_buf[dpos] = b'0' + ((d / 10) % 10) as u8; dpos += 1;
    date_buf[dpos] = b'0' + (d % 10) as u8; dpos += 1;
    let date_len = dpos;

    // Форматируем время HH:MM:SS
    let mut tpos = 0;
    let h = dt.hour as u32;
    time_buf[tpos] = b'0' + ((h / 10) % 10) as u8; tpos += 1;
    time_buf[tpos] = b'0' + (h % 10) as u8; tpos += 1;
    time_buf[tpos] = b':'; tpos += 1;
    let min = dt.minute as u32;
    time_buf[tpos] = b'0' + ((min / 10) % 10) as u8; tpos += 1;
    time_buf[tpos] = b'0' + (min % 10) as u8; tpos += 1;
    time_buf[tpos] = b':'; tpos += 1;
    let s = dt.second as u32;
    time_buf[tpos] = b'0' + ((s / 10) % 10) as u8; tpos += 1;
    time_buf[tpos] = b'0' + (s % 10) as u8; tpos += 1;
    let time_len = tpos;

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 6: ДАТА, ВРЕМЯ И ЧАСОВОЙ ПОЯС  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Текущее системное время получено из аппаратных часов (RTC/CMOS):");

    let label_col = 30usize;
    let field_col = 32usize;
    let field_w   = 16usize;

    draw_label(11, "Дата (ГГГГ-ММ-ДД)", label_col);
    draw_label(13, "Время (ЧЧ:ММ:СС)", label_col);

    draw_input_field(11, field_col, field_w, &date_buf, date_len);
    draw_input_field(13, field_col, field_w, &time_buf, time_len);

    goto(15, 10);
    wrs(FG_YELLOW); wrs(BOLD);
    wrs("Выберите системный часовой пояс (по умолчанию: Москва / МСК):");
    wrs(RESET);

    let mut items: [MenuItem; 9] = [
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
    ];
    for i in 0..TIMEZONES.len() {
        items[i].label = TIMEZONES[i].display;
    }

    draw_separator();
    draw_footer(" ↑↓ Выбор часового пояса    Enter Подтвердить    Esc Назад ");

    let sel = run_menu(16, 6, &items, 72)?;
    let tz = &TIMEZONES[sel];

    Some(tz.name)
}
