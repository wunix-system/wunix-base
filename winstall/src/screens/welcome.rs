// Стартовое меню и экран приветствия Wunix OS.

use crate::tui::{
    clear_screen, draw_box, draw_centered, draw_footer, draw_header, draw_logo,
    draw_separator, goto, hide_cursor, run_menu, wait_key, wrs, MenuItem, BOLD,
    FG_CYAN, FG_GREEN, FG_WHITE, FG_YELLOW, LOGO_ROW, RESET,
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
            label: "1. Установка Wunix OS  (разметка диска и установка на накопитель) ",
            disabled: false,
        },
        MenuItem {
            label: "2. Запуск Live-сессии (тестирование в оперативной памяти без установки)",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" ↑↓ / 1-2 Выбор    Enter Запуск ");

    let sel = run_menu(12, 6, &items, 72)?;
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
    draw_header("  ДОБРО ПОЖАЛОВАТЬ В WUNIX OS  ");
    draw_logo(LOGO_ROW);

    draw_box(8, 6, 68, 11);

    goto(9, 9);
    wrs(FG_GREEN); wrs(BOLD);
    wrs("Wunix OS — независимая 64-битная Unix-подобная операционная система");
    wrs(RESET);

    goto(11, 9);
    wrs(FG_WHITE);
    wrs("• Ядро написано на Rust и работает в Ring 3 с полной изоляцией процессов.");

    goto(12, 9);
    wrs("• Собственная подсистема VFS, драйверы VirtIO/ATA и сетевой стек (TCP/IP).");

    goto(13, 9);
    wrs("• Стандартный набор утилит командной строки POSIX и консольный шелл wsh.");

    goto(15, 9);
    wrs(FG_YELLOW);
    wrs("Благодарим вас за тестирование и участие в развитии системы Wunix!");
    wrs(RESET);

    goto(17, 9);
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
