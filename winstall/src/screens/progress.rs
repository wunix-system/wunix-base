// Отрисовка процесса разметки диска и прогресса копирования файлов системы.

use crate::tui::{
    center_col, char_count, clear_screen, draw_box, draw_centered, draw_footer,
    draw_header, draw_logo, draw_progress, draw_separator, goto, hide_cursor,
    write_u64, wr, wrs, BOLD, FG_CYAN, FG_GRAY, FG_GREEN, FG_WHITE, FG_YELLOW,
    LOGO_ROW, RESET,
};

/// Начальный экран процесса очистки и разметки диска
pub fn begin_partition_process_screen(disk_name: &str) {
    hide_cursor();
    clear_screen();
    draw_header("  ОЧИСТКА И РАЗМЕТКА ДИСКА  ");
    draw_logo(LOGO_ROW);

    let box_w = 74;
    let col = center_col(box_w);

    goto(9, col);
    wrs(FG_WHITE);
    wrs("Выполняется подготовка накопителя ");
    wrs(FG_YELLOW); wrs(BOLD); wrs(disk_name); wrs(RESET); wrs(FG_WHITE);
    wrs(" к установке:");

    draw_box(11, col, box_w, 8);

    goto(12, col + 3); wrs(FG_GRAY); wrs("[ ] 1. Очистка нулевого сектора (зачистка старых сигнатур)");
    goto(13, col + 3); wrs(FG_GRAY); wrs("[ ] 2. Запись новой таблицы разделов MBR");
    goto(14, col + 3); wrs(FG_GRAY); wrs("[ ] 3. Форматирование корневого раздела (FAT32)");
    goto(15, col + 3); wrs(FG_GRAY); wrs("[ ] 4. Инициализация раздела подкачки (SWAP)");
    goto(16, col + 3); wrs(FG_GRAY); wrs("[ ] 5. Синхронизация буферов и регистрация разделов ядра");

    draw_separator();
    draw_footer(" Пожалуйста, подождите завершения разметки ");
}

/// Обновление статуса шага разметки диска
pub fn update_partition_step(step: usize, completed: bool) {
    let box_w = 74;
    let col = center_col(box_w);
    let row = 11 + step;
    goto(row, col + 3);
    if completed {
        wrs(FG_GREEN); wrs(BOLD);
        wr(b"[\xe2\x9c\x93]");
        wrs(RESET);
    } else {
        wrs(FG_YELLOW); wrs(BOLD);
        wr(b"[\xe2\x96\xb6]");
        wrs(RESET);
    }
}

/// Начальный экран копирования компонентов Wunix Base
pub fn begin_install_progress_screen() {
    hide_cursor();
    clear_screen();
    draw_header("  УСТАНОВКА WUNIX BASE  ");
    draw_logo(LOGO_ROW);

    draw_centered(10, "Производится установка файлов системы на диск, пожалуйста подождите...");

    let box_w = 74;
    let col = center_col(box_w);
    draw_box(12, col, box_w, 7);

    draw_separator();
    draw_footer(" Не выключайте компьютер до завершения установки ");
}

/// Обновление индикатора прогресса установки файлов
pub fn update_install_progress(percent: u8, stage: &str, files_done: usize, files_total: usize) {
    let box_w = 74;
    let col = center_col(box_w);
    let box_right = col + box_w - 1;

    // Строка 14: Текущий этап установки
    goto(14, col + 3);
    wrs(FG_WHITE);
    wrs("Этап: ");
    wrs(stage);
    let st_chars = char_count(stage);
    let used_stage = 6 + st_chars;
    let max_inner = box_w.saturating_sub(6);
    let pad_stage = max_inner.saturating_sub(used_stage);
    for _ in 0..pad_stage { wr(b" "); }
    goto(14, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);

    // Строка 16: Индикатор прогресса
    draw_progress(16, col + 3, box_w.saturating_sub(18), percent);
    goto(16, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);

    // Строка 17: Количество скопированных файлов
    goto(17, col + 3);
    wrs(FG_GRAY);
    wrs("Компонентов: ");
    write_u64(files_done as u64);
    wr(b" / ");
    write_u64(files_total as u64);
    let mut done_digits = 1;
    let mut d = files_done;
    while d >= 10 { done_digits += 1; d /= 10; }
    let mut tot_digits = 1;
    let mut t = files_total;
    while t >= 10 { tot_digits += 1; t /= 10; }
    let used_comp = 13 + done_digits + 3 + tot_digits;
    let pad_comp = max_inner.saturating_sub(used_comp);
    for _ in 0..pad_comp { wr(b" "); }
    goto(17, box_right);
    wrs(FG_CYAN); wr(b"\xe2\x94\x82"); wrs(RESET);
}
