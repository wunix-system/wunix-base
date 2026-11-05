// Выбор режима разметки (автоматическая / ручная), отображение схемы и валидация.

use crate::disk::{
    validate_partition_plan, DiskInfo, PartitionPlan, MAX_PARTITIONS,
};
use crate::tui::{
    center_col, clear_screen, draw_box, draw_centered, draw_footer, draw_header,
    draw_logo, draw_separator, goto, hide_cursor, run_menu, write_u64, wr, wrs,
    MenuItem, BOLD, FG_CYAN, FG_GRAY, FG_GREEN, FG_RED, FG_WHITE, FG_YELLOW,
    LOGO_ROW, RESET,
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PartitionMode {
    Auto,
    Manual,
}

pub fn screen_partition_mode() -> Option<PartitionMode> {
    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 3: РАЗМЕТКА НАКОПИТЕЛЯ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Выберите метод разметки целевого жесткого диска:");

    let items = [
        MenuItem {
            label: "1. Автоматическая разметка  (рекомендуется: Boot + Swap + Root)       ",
            disabled: false,
        },
        MenuItem {
            label: "2. Ручная разметка разделов (выбор структуры разделов диска)          ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Подтвердить    ↑↓ Навигация    Esc Назад ");

    let menu_w = 70;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items, menu_w)?;
    Some(if sel == 0 {
        PartitionMode::Auto
    } else {
        PartitionMode::Manual
    })
}

/// Экран ручной настройки разделов
pub fn screen_partition_manual(
    disk: &DiskInfo,
    selected_fs: &'static str,
    plans: &mut [PartitionPlan; MAX_PARTITIONS],
) -> Option<usize> {
    hide_cursor();
    clear_screen();
    draw_header("  РУЧНАЯ РАЗМЕТКА РАЗДЕЛОВ  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Настройте структуру разделов диска (выберите предустановленную схему):");

    let total_mb = disk.size_mib();

    let items = [
        MenuItem {
            label: "1. Стандартная схема: /boot (64M) + swap (256M) + / (остаток)",
            disabled: false,
        },
        MenuItem {
            label: "2. Расширенная схема: /boot (128M) + swap (512M) + / (остаток)",
            disabled: total_mb < 800,
        },
        MenuItem {
            label: "3. Простая схема:     без swap, весь диск под корень (/)      ",
            disabled: false,
        },
    ];

    draw_separator();
    draw_footer(" Enter Выбрать схему    ↑↓ Навигация    Esc Назад ");

    let menu_w = 68;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items, menu_w)?;

    match sel {
        0 => {
            // Стандартная схема
            let boot_sec: u32 = 131072; // 64 MiB
            let swap_sec: u32 = 524288; // 256 MiB
            let root_sec = (disk.sector_count.saturating_sub(2048 + (boot_sec + swap_sec) as u64)).min(0xFFFF_FFFF) as u32;

            plans[0] = PartitionPlan {
                num: 1, mount_point: "/boot", fstype: "fat32", start_lba: 2048,
                sector_count: boot_sec, bootable: true,
            };
            plans[1] = PartitionPlan {
                num: 2, mount_point: "swap", fstype: "swap", start_lba: 2048 + boot_sec,
                sector_count: swap_sec, bootable: false,
            };
            plans[2] = PartitionPlan {
                num: 3, mount_point: "/", fstype: selected_fs, start_lba: 2048 + boot_sec + swap_sec,
                sector_count: root_sec, bootable: false,
            };
            Some(3)
        }
        1 => {
            // Расширенная схема
            let boot_sec: u32 = 262144; // 128 MiB
            let swap_sec: u32 = 1048576; // 512 MiB
            let root_sec = (disk.sector_count.saturating_sub(2048 + (boot_sec + swap_sec) as u64)).min(0xFFFF_FFFF) as u32;

            plans[0] = PartitionPlan {
                num: 1, mount_point: "/boot", fstype: "fat32", start_lba: 2048,
                sector_count: boot_sec, bootable: true,
            };
            plans[1] = PartitionPlan {
                num: 2, mount_point: "swap", fstype: "swap", start_lba: 2048 + boot_sec,
                sector_count: swap_sec, bootable: false,
            };
            plans[2] = PartitionPlan {
                num: 3, mount_point: "/", fstype: selected_fs, start_lba: 2048 + boot_sec + swap_sec,
                sector_count: root_sec, bootable: false,
            };
            Some(3)
        }
        _ => {
            // Простая схема без swap
            let root_sec = (disk.sector_count.saturating_sub(2048)).min(0xFFFF_FFFF) as u32;
            plans[0] = PartitionPlan {
                num: 1, mount_point: "/", fstype: selected_fs, start_lba: 2048,
                sector_count: root_sec, bootable: true,
            };
            Some(1)
        }
    }
}

/// Экран подтверждения разметки с РЕАЛЬНОЙ проверкой инвариантов и выводом предупреждений
pub fn screen_partition_confirm(
    disk_name: &str,
    plans: &[PartitionPlan],
    count: usize,
) -> bool {
    let val = validate_partition_plan(plans, count);

    hide_cursor();
    clear_screen();
    draw_header("  ПОДТВЕРЖДЕНИЕ И ВАЛИДАЦИЯ РАЗМЕТКИ  ");
    draw_logo(LOGO_ROW);

    let box_w = 78;
    let col = center_col(box_w);

    goto(9, col);
    wrs(FG_WHITE);
    wrs("План разметки накопителя ");
    wrs(FG_YELLOW); wrs(BOLD); wrs(disk_name); wrs(RESET); wrs(FG_WHITE);
    wrs(" (MBR таблица разделов):");

    // Таблица разделов
    draw_box(10, col, box_w, 2 + count);
    goto(11, col + 2);
    wrs(FG_CYAN); wrs(BOLD);
    wrs("№   Точка монтирования   Тип ФС     Размер      Загрузочный (Boot)");
    wrs(RESET);

    for i in 0..count {
        let p = &plans[i];
        let row = 12 + i;
        goto(row, col + 2);
        wrs(FG_WHITE);
        write_u64((i + 1) as u64);
        wr(b"   ");

        // Точка монтирования
        let mp = if p.mount_point.is_empty() { "-" } else { p.mount_point };
        wrs(mp);
        for _ in mp.len()..21 { wr(b" "); }

        // Тип ФС
        wrs(p.fstype);
        for _ in p.fstype.len()..11 { wr(b" "); }

        // Размер
        let mb = p.size_mib();
        write_u64(mb);
        wrs(" MiB");
        let mut d = mb;
        let mut digs = 1;
        while d >= 10 { digs += 1; d /= 10; }
        for _ in (digs + 4)..12 { wr(b" "); }

        // Boot
        if p.bootable {
            wrs(FG_GREEN); wrs(BOLD); wrs("Да  [Active]"); wrs(RESET);
        } else {
            wrs(FG_GRAY); wrs("Нет"); wrs(RESET);
        }
    }

    // Блок предупреждений и проверки (Validation Warnings)
    let warn_row = 12 + count + 1;
    let mut cur_row = warn_row;

    if !val.has_root {
        goto(cur_row, col);
        wrs(FG_RED); wrs(BOLD);
        wrs("✖ ОШИБКА: Корневой раздел (/) не назначен! Установка невозможна.");
        wrs(RESET);
        cur_row += 1;
    }

    if !val.has_swap {
        goto(cur_row, col);
        wrs(FG_YELLOW); wrs(BOLD);
        wrs("▲ ВНИМАНИЕ: Раздел подкачки (swap) отсутствует (риск нехватки памяти).");
        wrs(RESET);
        cur_row += 1;
    }

    if !val.has_boot {
        goto(cur_row, col);
        wrs(FG_YELLOW);
        wrs("▲ ПРЕДУПРЕЖДЕНИЕ: Загрузочный флаг не установлен (MBR Bootable).");
        wrs(RESET);
        cur_row += 1;
    }

    draw_separator();

    if !val.is_valid {
        draw_footer(" Esc Вернуться назад и исправить разметку ");
        crate::tui::wait_key();
        return false;
    }

    let items = [
        MenuItem {
            label: "1. Применить разметку к диску (все старые данные будут стёрты)",
            disabled: false,
        },
        MenuItem {
            label: "2. Вернуться назад к настройке разделов                        ",
            disabled: false,
        },
    ];

    draw_footer(" Enter Подтвердить    ↑↓ Навигация    Esc Назад ");

    let menu_w = 66;
    let menu_col = center_col(menu_w);
    let menu_row = cur_row.max(16);
    match run_menu(menu_row, menu_col, &items, menu_w) {
        Some(0) => true,
        _ => false,
    }
}
