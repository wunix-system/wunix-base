// Окно выбора файловой системы для установки Wunix OS.

use crate::tui::{
    center_col, clear_screen, draw_centered, draw_footer, draw_header, draw_logo,
    draw_separator, hide_cursor, run_menu, MenuItem, LOGO_ROW,
};

pub fn screen_select_fs() -> Option<&'static str> {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 2: ВЫБОР ФАЙЛОВОЙ СИСТЕМЫ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Выберите тип файловой системы для корневого раздела (/):");

    let items = [
        MenuItem {
            label: "1. FAT32 — Основная файловая система Wunix OS (Рекомендуется)      ",
            disabled: false,
        },
        MenuItem {
            label: "2. EXT4  — Журналируемая файловая система (в разработке)           ",
            disabled: true,
        },
        MenuItem {
            label: "3. Btrfs — CoW файловая система нового поколения (в разработке)    ",
            disabled: true,
        },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    ↑↓ Навигация    Esc Назад ");

    let menu_w = 68;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items, menu_w)?;
    match sel {
        0 => Some("fat32"),
        1 => Some("ext4"),
        _ => Some("btrfs"),
    }
}
