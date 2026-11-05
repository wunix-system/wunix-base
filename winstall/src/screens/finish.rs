// Экран успешного завершения установки Wunix OS.

use crate::tui::{
    center_col, clear_screen, draw_box, draw_centered, draw_footer, draw_header,
    draw_info, draw_logo, draw_separator, goto, hide_cursor, run_menu_compact,
    wait_key, wrs, MenuItem, BOLD, FG_GRAY, FG_GREEN, FG_RED, FG_WHITE, LOGO_ROW,
    RESET,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FinishChoice {
    Reboot,
    Shutdown,
    Shell,
}

pub fn screen_finish(
    disk: &str,
    username: &str,
    hostname: &str,
    ip: &str,
    tz: &str,
) -> FinishChoice {
    hide_cursor();
    clear_screen();
    draw_header("  УСТАНОВКА УСПЕШНО ЗАВЕРШЕНА  ");
    draw_logo(LOGO_ROW);

    draw_info(9, "✓  Система Wunix OS успешно установлена на целевой накопитель!", FG_GREEN);

    let info_w = 60;
    let col = center_col(info_w);

    goto(10, col); wrs(FG_GRAY); wrs("Целевой накопитель   : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(disk); wrs(RESET);
    goto(11, col); wrs(FG_GRAY); wrs("Пользователь (sudo)  : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(username); wrs(RESET);
    goto(12, col); wrs(FG_GRAY); wrs("Имя хоста (Hostname) : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(hostname); wrs(RESET);
    goto(13, col); wrs(FG_GRAY); wrs("Сетевой IP-адрес     : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(ip); wrs(RESET);
    goto(14, col); wrs(FG_GRAY); wrs("Часовой пояс (TZ)    : "); wrs(RESET); wrs(FG_WHITE); wrs(BOLD); wrs(tz); wrs(RESET);

    draw_centered(16, "Благодарим вас за тестирование Wunix OS!");
    draw_centered(17, "Извлеките установочный ISO-образ и перезагрузите компьютер.");

    let items = [
        MenuItem { label: "1. Перезагрузить систему (загрузка с установленного диска)", disabled: false },
        MenuItem { label: "2. Выключить компьютер                                    ", disabled: false },
        MenuItem { label: "3. Выйти в Live командную оболочку (wsh)                  ", disabled: false },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    ↑↓ Навигация ");

    let menu_w = 64;
    let menu_col = center_col(menu_w);
    match run_menu_compact(19, menu_col, &items, menu_w) {
        Some(0) => FinishChoice::Reboot,
        Some(1) => FinishChoice::Shutdown,
        _ => FinishChoice::Shell,
    }
}

pub fn screen_error(msg: &str) {
    hide_cursor();
    clear_screen();
    draw_header("  ОШИБКА  ");
    draw_logo(LOGO_ROW);

    let box_w = 70;
    let col = center_col(box_w);
    draw_box(13, col, box_w, 5);
    goto(15, col + 4);
    wrs(FG_RED); wrs(BOLD);
    wrs(msg);
    wrs(RESET);

    draw_info(19, "Нажмите любую клавишу...", FG_GRAY);
    draw_separator();
    draw_footer(" Нажмите любую клавишу ");
    wait_key();
}
