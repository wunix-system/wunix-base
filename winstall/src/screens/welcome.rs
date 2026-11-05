// Стартовое меню и экран приветствия Wunix OS.

use crate::tui::{
    center_col, clear_screen, draw_box, draw_centered, draw_footer, draw_header,
    draw_logo, draw_separator, goto, hide_cursor, run_menu, wait_key, wrs, MenuItem,
    BOLD, FG_CYAN, FG_GREEN, FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WelcomeChoice {
    Install,
    Live,
}

/// Стартовое загрузочное меню: ПЕРВЫМ пунктом идет Установка!
pub fn screen_boot_menu() -> Option<WelcomeChoice> {
    hide_cursor();
    clear_screen();
    draw_header("  ЗАГРУЗОЧНОЕ МЕНЮ WUNIX OS  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Добро пожаловать в Wunix OS!");
    draw_centered(10, "Выберите режим запуска операционной системы:");

    let items = [
        MenuItem {
            label: "1. Установка Wunix OS  (разметка накопителя и установка на диск) ",
            disabled: false,
        },
        MenuItem {
            label: "2. Запуск Live-сессии (ознакомительный режим в оперативной памяти)",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Запуск    ↑↓ / 1-2 Выбор ");

    let menu_w = 68;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items, menu_w)?;
    Some(if sel == 0 {
        WelcomeChoice::Install
    } else {
        WelcomeChoice::Live
    })
}

/// Окно приветствия: рассказ про Wunix OS, благодарность за тестирование и этапы установки
pub fn screen_welcome_info() -> bool {
    hide_cursor();
    clear_screen();
    draw_header("  МАСТЕР УСТАНОВКИ WUNIX OS  ");
    draw_logo(LOGO_ROW);

    let box_w = 78;
    let box_h = 11;
    let col = center_col(box_w);

    draw_box(8, col, box_w, box_h);

    goto(9, col + 3);
    wrs(FG_GREEN); wrs(BOLD);
    wrs("Wunix OS — современная компактная операционная система с открытым кодом.");
    wrs(RESET);

    goto(11, col + 3);
    wrs(FG_WHITE);
    wrs("• Надежная архитектура: модульное ядро, сетевой стек и подсистема VFS.");

    goto(12, col + 3);
    wrs("• Стандартные утилиты командной строки POSIX и консольная оболочка wsh.");

    goto(14, col + 3);
    wrs(FG_YELLOW);
    wrs("Благодарим вас за тестирование и участие в развитии системы Wunix!");
    wrs(RESET);

    goto(16, col + 3);
    wrs(FG_CYAN);
    wrs("Мастер установки поможет разметить диск, настроить сеть и развернуть ОС.");
    wrs(RESET);

    draw_separator();
    draw_footer(" Enter Начать установку    Esc Выход в Live-сессию ");

    match wait_key() {
        crate::tui::Key::Esc => false,
        _ => true,
    }
}
