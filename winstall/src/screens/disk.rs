// Окно выбора целевого диска для установки Wunix OS.

use crate::disk::DiskInfo;
use crate::tui::{
    center_col, clear_screen, draw_centered, draw_footer, draw_header, draw_logo,
    draw_separator, goto, hide_cursor, run_menu, u64_to_buf, wait_key, wrs, MenuItem,
    BOLD, FG_RED, FG_WHITE, LOGO_ROW, RESET,
};

pub fn screen_select_disk<'a>(disks: &'a [DiskInfo], count: usize) -> Option<&'a DiskInfo> {
    if count == 0 {
        hide_cursor();
        clear_screen();
        draw_header("  ОШИБКА: НАКОПИТЕЛИ НЕ ОБНАРУЖЕНЫ  ");
        draw_logo(LOGO_ROW);

        let col = center_col(64);
        goto(11, col);
        wrs(FG_RED); wrs(BOLD);
        wrs("В системе не найдено ни одного блочного устройства (диска)!");
        wrs(RESET);

        goto(13, col);
        wrs(FG_WHITE);
        wrs("Проверьте подключение жесткого диска к виртуальной машине");
        goto(14, col);
        wrs("или параметры эмулятора (например, -drive file=disk.img).");

        draw_separator();
        draw_footer(" Enter Повторить попытку    Esc Выйти в Live-консоль ");
        wait_key();
        return None;
    }

    let mut label_bufs = [[0u8; 64]; 8];
    let mut label_lens = [0usize; 8];
    let real = count.min(8);

    for i in 0..real {
        let d = &disks[i];
        let name = d.name().as_bytes();
        let mut pos = 0;
        let nlen = name.len().min(14);
        label_bufs[i][pos..pos + nlen].copy_from_slice(&name[..nlen]);
        pos += nlen;
        while pos < 14 { label_bufs[i][pos] = b' '; pos += 1; }

        let size_mb = d.size_mib();
        if size_mb >= 1024 {
            let gib = size_mb / 1024;
            let frac = (size_mb % 1024) * 10 / 1024;
            pos += u64_to_buf(&mut label_bufs[i][pos..], gib);
            label_bufs[i][pos] = b'.'; pos += 1;
            pos += u64_to_buf(&mut label_bufs[i][pos..], frac);
            let sfx = " GiB [Накопитель]                        ".as_bytes();
            let slen = sfx.len().min(label_bufs[i].len() - pos);
            label_bufs[i][pos..pos + slen].copy_from_slice(&sfx[..slen]);
            pos += slen;
        } else {
            pos += u64_to_buf(&mut label_bufs[i][pos..], size_mb);
            let sfx = " MiB [Накопитель]                        ".as_bytes();
            let slen = sfx.len().min(label_bufs[i].len() - pos);
            label_bufs[i][pos..pos + slen].copy_from_slice(&sfx[..slen]);
            pos += slen;
        }
        label_lens[i] = pos;
    }

    hide_cursor();
    clear_screen();
    draw_header("  ЭТАП 1: ВЫБОР ЦЕЛЕВОГО ДИСКА  ");
    draw_logo(LOGO_ROW);

    draw_centered(9, "Выберите накопитель, на который будет установлена Wunix OS:");
    draw_centered(10, "Внимание: выбранный накопитель будет размечен заново!");

    let mut items: [MenuItem; 8] = [
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
        MenuItem { label: "", disabled: false },
    ];
    for i in 0..real {
        items[i].label = core::str::from_utf8(&label_bufs[i][..label_lens[i]]).unwrap_or("?");
    }

    draw_separator();
    draw_footer(" Enter Выбрать диск    ↑↓ Навигация    Esc Назад ");

    let menu_w = 60;
    let col = center_col(menu_w);
    let sel = run_menu(12, col, &items[..real], menu_w)?;
    Some(&disks[sel])
}
