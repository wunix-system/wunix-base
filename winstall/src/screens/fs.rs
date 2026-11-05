// Окно выбора файловой системы для установки Wunix OS.

use crate::tui::{
    clear_screen, draw_centered, draw_footer, draw_header, draw_logo, draw_separator,
    hide_cursor, run_menu, MenuItem, LOGO_ROW,
};

pub fn screen_select_fs() -> Option<&'static str> {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 2: ВЫБОР ФАЙЛОВОЙ СИСТЕМЫ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Выберите тип файловой системы для корневого раздела (/):");

    let items = [
        MenuItem {
            label: "1. FAT32 — Штатная файловая система Wunix OS (Limine/UEFI) [Рекомендуется]",
            disabled: false,
        },
        MenuItem {
            label: "2. EXT4  — Linux Journaling FS (в разработке для Stage 15)                ",
            disabled: true,
        },
        MenuItem {
            label: "3. Btrfs — CoW Next-Gen FS    (в разработке для Stage 15)                ",
            disabled: true,
        },
    ];

    draw_separator();
    draw_footer(" ↑↓ Выбор    Enter Подтвердить    Esc Назад ");

    let sel = run_menu(12, 5, &items, 72)?;
    match sel {
        0 => Some("fat32"),
        1 => Some("ext4"),
        _ => Some("btrfs"),
    }
}
