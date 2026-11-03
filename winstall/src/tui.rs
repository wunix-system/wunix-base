// Низкоуровневые TUI-примитивы для winstall.
// Принцип: каждый экран = четкая сетка, точный подсчет символов Unicode.

use libwunix::{nanosleep, read, write, STDIN_FILENO, STDOUT_FILENO};

pub const W: usize = 80;
pub const LOGO_ROW: usize = 3;

// ANSI-коды цветов и атрибутов
pub const RESET: &str     = "\x1b[0m";
pub const BOLD: &str      = "\x1b[1m";
pub const DIM: &str       = "\x1b[2m";
pub const FG_CYAN: &str   = "\x1b[36m";
pub const FG_WHITE: &str  = "\x1b[97m";
pub const FG_GRAY: &str   = "\x1b[90m";
pub const FG_GREEN: &str  = "\x1b[32m";
pub const FG_RED: &str    = "\x1b[31m";
pub const FG_YELLOW: &str = "\x1b[33m";
pub const BG_BLUE: &str   = "\x1b[44m";
pub const BG_GREEN: &str  = "\x1b[42m";
pub const BG_DARK: &str   = "\x1b[40m";

#[inline] pub fn wr(s: &[u8]) { write(STDOUT_FILENO, s); }
#[inline] pub fn wrs(s: &str) { write(STDOUT_FILENO, s.as_bytes()); }

/// Подсчет визуальных символов в строке UTF-8 (для предотвращения смещений при кириллице)
#[inline]
pub fn char_count(s: &str) -> usize {
    s.chars().count()
}

pub fn goto(r: usize, c: usize) {
    wr(b"\x1b[");
    write_dec(r as u16);
    wr(b";");
    write_dec(c as u16);
    wr(b"H");
}

pub fn clear_screen()  { wr(b"\x1b[2J\x1b[H"); }
pub fn hide_cursor()   { wr(b"\x1b[?25l"); }
pub fn show_cursor()   { wr(b"\x1b[?25h"); }
pub fn clear_eol()     { wr(b"\x1b[K"); }

fn write_dec(mut n: u16) {
    if n == 0 { wr(b"0"); return; }
    let mut buf = [0u8; 5];
    let mut pos = 5;
    while n > 0 { pos -= 1; buf[pos] = b'0' + (n % 10) as u8; n /= 10; }
    wr(&buf[pos..]);
}

// ─── Клавиши ─────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    Tab,
    Esc,
    CtrlC,
    Char(u8),
    Unknown,
}

pub fn read_key() -> Key {
    let mut b = [0u8; 1];
    let n = read(STDIN_FILENO, &mut b) as usize;
    if n == 0 {
        return Key::Unknown;
    }

    if b[0] == b'\x1b' {
        // Проверяем, является ли это последовательностью стрелок ANSI: \x1b[A, \x1b[B, \x1bOA...
        let mut seq = [0u8; 2];
        let mut m = read(STDIN_FILENO, &mut seq) as usize;
        if m == 0 {
            // Для медленных последовательностей терминала проверяем с небольшой паузой
            for _ in 0..5 {
                sleep_ms(3);
                m = read(STDIN_FILENO, &mut seq) as usize;
                if m > 0 { break; }
            }
        }

        if m >= 2 && (seq[0] == b'[' || seq[0] == b'O' || seq[0] == b'o') {
            return match seq[1] {
                b'A' => Key::Up,
                b'B' => Key::Down,
                b'C' => Key::Right,
                b'D' => Key::Left,
                _ => Key::Unknown,
            };
        }
        return Key::Esc;
    }

    match b[0] {
        b'\r' | b'\n' => Key::Enter,
        b'\t' => Key::Tab,
        b'\x03' => Key::CtrlC,
        b'\x08' | b'\x7f' => Key::Backspace,
        ch => Key::Char(ch),
    }
}

pub fn wait_key() -> Key {
    read_key()
}

pub fn sleep_ms(ms: u64) {
    let ns = ms * 1_000_000;
    let req = [0u64, ns];
    let _ = nanosleep(req.as_ptr() as *const u8, core::ptr::null_mut());
}

// ─── Шапка и подвал ──────────────────────────────────────────────────────────

pub fn draw_header(title: &str) {
    goto(1, 1);
    wrs(BG_BLUE); wrs(BOLD); wrs(FG_WHITE);
    let tc = char_count(title);
    let pad = W.saturating_sub(tc) / 2;
    for _ in 0..pad { wr(b" "); }
    wrs(title);
    for _ in 0..W.saturating_sub(tc + pad) { wr(b" "); }
    wrs(RESET);
}

pub fn draw_separator() {
    goto(20, 1);
    wrs(FG_GRAY);
    // Горизонтальная линия U+2500 (─)
    for _ in 0..W {
        wr(b"\xe2\x94\x80");
    }
    wrs(RESET);
}

pub fn draw_footer(hint: &str) {
    goto(22, 1);
    wrs(DIM); wrs(FG_GRAY);
    let hc = char_count(hint);
    let pad = W.saturating_sub(hc) / 2;
    for _ in 0..pad { wr(b" "); }
    wrs(hint);
    for _ in 0..W.saturating_sub(hc + pad) { wr(b" "); }
    wrs(RESET);
}

// ─── Логотип Wunix (монолитные блоки █, без тонких рамок) ─────────────────────

const LOGO: &[&str] = &[
    "\u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}",
    "\u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}  \u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588} ",
    "\u{2588}\u{2588} \u{2588}\u{2588} \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588} \u{2588}\u{2588} \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}\u{2588}\u{2588}  ",
    "\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}  \u{2588}\u{2588}  \u{2588}\u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}   \u{2588}\u{2588}  \u{2588}\u{2588} ",
    " \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}   \u{2588}\u{2588}   \u{2588}\u{2588}\u{2588}  \u{2588}\u{2588}  \u{2588}\u{2588}    \u{2588}\u{2588}",
];

const LOGO_VISUAL_W: usize = 42;

pub fn draw_logo(start_row: usize) {
    let col = 1 + (W.saturating_sub(LOGO_VISUAL_W)) / 2;
    wrs(BOLD); wrs(FG_CYAN);
    for (i, line) in LOGO.iter().enumerate() {
        goto(start_row + i, col);
        wrs(line);
    }
    wrs(RESET);
}

// ─── Рамка ───────────────────────────────────────────────────────────────────

pub fn draw_box(row: usize, col: usize, width: usize, height: usize) {
    wrs(FG_CYAN);
    goto(row, col);
    wr(b"\xe2\x94\x8c");
    for _ in 0..width.saturating_sub(2) { wr(b"\xe2\x94\x80"); }
    wr(b"\xe2\x94\x90");
    for r in (row + 1)..(row + height.saturating_sub(1)) {
        goto(r, col); wr(b"\xe2\x94\x82");
        goto(r, col + width - 1); wr(b"\xe2\x94\x82");
    }
    goto(row + height - 1, col);
    wr(b"\xe2\x94\x94");
    for _ in 0..width.saturating_sub(2) { wr(b"\xe2\x94\x80"); }
    wr(b"\xe2\x94\x98");
    wrs(RESET);
}

// ─── Меню ────────────────────────────────────────────────────────────────────

pub struct MenuItem<'a> {
    pub label: &'a str,
    pub disabled: bool,
}

pub fn draw_menu(
    start_row: usize,
    col: usize,
    items: &[MenuItem<'_>],
    selected: usize,
    item_width: usize,
) {
    for (i, item) in items.iter().enumerate() {
        goto(start_row + i * 2, col);
        let label_chars = char_count(item.label);
        if item.disabled {
            wrs(FG_GRAY); wrs(DIM);
            wr(b"    ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in used..item_width { wr(b" "); }
            wrs(RESET);
        } else if i == selected {
            wrs(BG_BLUE); wrs(BOLD); wrs(FG_WHITE);
            // Индикатор курсора: ▶
            wr(b" \xe2\x96\xb6  ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in used..item_width { wr(b" "); }
            wrs(RESET);
        } else {
            wrs(RESET);
            wr(b"    ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in used..item_width { wr(b" "); }
        }
    }
}

/// Интерактивное меню: навигация стрелками ↑↓, Tab, клавишами 1-9, Vim (j/k), WASD (w/s).
pub fn run_menu(
    start_row: usize,
    col: usize,
    items: &[MenuItem<'_>],
    item_width: usize,
) -> Option<usize> {
    let count = items.len();
    if count == 0 { return None; }
    let mut sel = items.iter().position(|it| !it.disabled).unwrap_or(0);
    draw_menu(start_row, col, items, sel, item_width);
    loop {
        match read_key() {
            Key::Up
            | Key::Char(b'k')
            | Key::Char(b'K')
            | Key::Char(b'w')
            | Key::Char(b'W') => {
                let mut n = sel;
                loop {
                    if n == 0 {
                        n = count - 1;
                    } else {
                        n -= 1;
                    }
                    if !items[n].disabled {
                        sel = n;
                        break;
                    }
                    if n == sel { break; }
                }
                draw_menu(start_row, col, items, sel, item_width);
            }
            Key::Down
            | Key::Tab
            | Key::Char(b'j')
            | Key::Char(b'J')
            | Key::Char(b's')
            | Key::Char(b'S') => {
                let mut n = sel;
                loop {
                    if n + 1 >= count {
                        n = 0;
                    } else {
                        n += 1;
                    }
                    if !items[n].disabled {
                        sel = n;
                        break;
                    }
                    if n == sel { break; }
                }
                draw_menu(start_row, col, items, sel, item_width);
            }
            Key::Char(b) if b >= b'1' && b <= b'9' => {
                let idx = (b - b'1') as usize;
                if idx < count && !items[idx].disabled {
                    sel = idx;
                    draw_menu(start_row, col, items, sel, item_width);
                }
            }
            Key::Char(b' ') | Key::Enter => {
                if !items[sel].disabled { return Some(sel); }
            }
            Key::Esc | Key::CtrlC => return None,
            _ => {}
        }
    }
}

pub fn draw_menu_compact(
    start_row: usize,
    col: usize,
    items: &[MenuItem<'_>],
    selected: usize,
    item_width: usize,
) {
    for (i, item) in items.iter().enumerate() {
        goto(start_row + i, col);
        let label_chars = char_count(item.label);
        if item.disabled {
            wrs(FG_GRAY); wrs(DIM);
            wr(b"    ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in 0..item_width.saturating_sub(used) { wr(b" "); }
            wrs(RESET);
        } else if i == selected {
            wrs(BG_BLUE); wrs(BOLD); wrs(FG_WHITE);
            // Индикатор курсора: ▶
            wr(b" \xe2\x96\xb6  ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in 0..item_width.saturating_sub(used) { wr(b" "); }
            wrs(RESET);
        } else {
            wrs(RESET);
            wr(b"    ");
            wrs(item.label);
            let used = 4 + label_chars;
            for _ in 0..item_width.saturating_sub(used) { wr(b" "); }
        }
    }
}

/// Компактное интерактивное меню (без пустых строк между пунктами, шаг = 1 строка).
pub fn run_menu_compact(
    start_row: usize,
    col: usize,
    items: &[MenuItem<'_>],
    item_width: usize,
) -> Option<usize> {
    let count = items.len();
    if count == 0 { return None; }
    let mut sel = items.iter().position(|it| !it.disabled).unwrap_or(0);
    draw_menu_compact(start_row, col, items, sel, item_width);
    loop {
        match read_key() {
            Key::Up
            | Key::Char(b'k')
            | Key::Char(b'K')
            | Key::Char(b'w')
            | Key::Char(b'W') => {
                let mut n = sel;
                loop {
                    if n == 0 {
                        n = count - 1;
                    } else {
                        n -= 1;
                    }
                    if !items[n].disabled {
                        sel = n;
                        break;
                    }
                    if n == sel { break; }
                }
                draw_menu_compact(start_row, col, items, sel, item_width);
            }
            Key::Down
            | Key::Tab
            | Key::Char(b'j')
            | Key::Char(b'J')
            | Key::Char(b's')
            | Key::Char(b'S') => {
                let mut n = sel;
                loop {
                    if n + 1 >= count {
                        n = 0;
                    } else {
                        n += 1;
                    }
                    if !items[n].disabled {
                        sel = n;
                        break;
                    }
                    if n == sel { break; }
                }
                draw_menu_compact(start_row, col, items, sel, item_width);
            }
            Key::Char(b) if b >= b'1' && b <= b'9' => {
                let idx = (b - b'1') as usize;
                if idx < count && !items[idx].disabled {
                    sel = idx;
                    draw_menu_compact(start_row, col, items, sel, item_width);
                }
            }
            Key::Char(b' ') | Key::Enter => {
                if !items[sel].disabled { return Some(sel); }
            }
            Key::Esc | Key::CtrlC => return None,
            _ => {}
        }
    }
}

// ─── Поле ввода ──────────────────────────────────────────────────────────────

pub fn draw_input_field(row: usize, col: usize, width: usize, buf: &[u8], len: usize) {
    goto(row, col);
    wrs(BG_DARK); wrs(FG_WHITE); wrs(BOLD);
    wr(b" ");
    wr(&buf[..len]);
    let vlen = core::str::from_utf8(&buf[..len])
        .map(|s| s.chars().count())
        .unwrap_or(len);
    for _ in vlen..width.saturating_sub(1) { wr(b" "); }
    wrs(RESET);
}

pub fn draw_password_field(row: usize, col: usize, width: usize, len: usize) {
    goto(row, col);
    wrs(BG_DARK); wrs(FG_WHITE); wrs(BOLD);
    wr(b" ");
    for _ in 0..len { wr(b"*"); }
    for _ in len..width.saturating_sub(1) { wr(b" "); }
    wrs(RESET);
}

pub fn input_text(row: usize, col: usize, buf: &mut [u8], field_width: usize) -> usize {
    let max = (field_width.saturating_sub(2)).min(buf.len().saturating_sub(1));
    let mut len = 0usize;
    while len < max && buf[len] != 0 {
        len += 1;
    }
    loop {
        hide_cursor();
        draw_input_field(row, col, field_width, buf, len);
        let vlen = core::str::from_utf8(&buf[..len])
            .map(|s| s.chars().count())
            .unwrap_or(len);
        goto(row, col + 1 + vlen);
        show_cursor();

        let key = read_key();
        hide_cursor();

        match key {
            Key::Enter => break,
            Key::Esc | Key::CtrlC => { len = 0; break; }
            Key::Backspace => {
                if len > 0 {
                    while len > 0 && (buf[len - 1] & 0xC0) == 0x80 {
                        len -= 1;
                        buf[len] = 0;
                    }
                    if len > 0 {
                        len -= 1;
                        buf[len] = 0;
                    }
                }
            }
            Key::Char(ch) if ch >= 0x20 && ch != 0x7f => {
                if len < max { buf[len] = ch; len += 1; }
            }
            _ => {}
        }
    }
    draw_input_field(row, col, field_width, buf, len);
    len
}

pub fn input_password(row: usize, col: usize, buf: &mut [u8], field_width: usize) -> usize {
    let max = (field_width.saturating_sub(2)).min(buf.len().saturating_sub(1));
    let mut len = 0usize;
    loop {
        hide_cursor();
        draw_password_field(row, col, field_width, len);
        goto(row, col + 1 + len);
        show_cursor();

        let key = read_key();
        hide_cursor();

        match key {
            Key::Enter => break,
            Key::Esc | Key::CtrlC => { len = 0; break; }
            Key::Backspace => {
                if len > 0 {
                    while len > 0 && (buf[len - 1] & 0xC0) == 0x80 {
                        len -= 1;
                        buf[len] = 0;
                    }
                    if len > 0 {
                        len -= 1;
                        buf[len] = 0;
                    }
                }
            }
            Key::Char(ch) if ch >= 0x20 && ch != 0x7f => {
                if len < max { buf[len] = ch; len += 1; }
            }
            _ => {}
        }
    }
    draw_password_field(row, col, field_width, len);
    len
}

// ─── Прогресс-бар ────────────────────────────────────────────────────────────

pub fn draw_progress(row: usize, col: usize, bar_w: usize, percent: u8) {
    let fill = (bar_w * percent.min(100) as usize) / 100;
    goto(row, col);
    wrs(FG_CYAN); wr(b"[");
    wrs(BG_GREEN); wrs(FG_GREEN);
    for _ in 0..fill { wr(b"\xe2\x96\x88"); }   // █
    wrs(RESET); wrs(FG_GRAY);
    for _ in fill..bar_w { wr(b"\xe2\x96\x91"); } // ░
    wrs(RESET); wrs(FG_CYAN); wr(b"]"); wrs(RESET);
    wr(b"  ");
    if percent < 10 { wr(b"  "); }
    else if percent < 100 { wr(b" "); }
    write_dec(percent as u16);
    wr(b"%");
}

// ─── Вспомогательные ─────────────────────────────────────────────────────────

pub fn draw_label(row: usize, label: &str, value_col: usize) {
    let lc = char_count(label);
    let label_col = value_col.saturating_sub(lc + 3);
    goto(row, label_col);
    wrs(FG_GRAY); wrs(label); wr(b" : "); wrs(RESET);
}

pub fn draw_centered(row: usize, text: &str) {
    let tc = char_count(text);
    let col = 1 + W.saturating_sub(tc) / 2;
    goto(row, 1); wr(b"\x1b[2K");
    goto(row, col); wrs(text);
}

pub fn draw_info(row: usize, msg: &str, color: &str) {
    goto(row, 1); wr(b"\x1b[2K");
    let mc = char_count(msg);
    let col = 1 + W.saturating_sub(mc) / 2;
    goto(row, col); wrs(color); wrs(BOLD); wrs(msg); wrs(RESET);
}

pub fn draw_status(row: usize, col: usize, ok: bool, msg: &str) {
    goto(row, col);
    if ok { wrs(FG_GREEN); wrs(BOLD); wr(b"\xe2\x9c\x93  "); }
    else  { wrs(FG_RED);   wrs(BOLD); wr(b"\xe2\x9c\x97  "); }
    wrs(RESET); wrs(msg); clear_eol();
}

pub fn write_u64(mut n: u64) {
    if n == 0 { wr(b"0"); return; }
    let mut tmp = [0u8; 20];
    let mut pos = 20;
    while n > 0 { pos -= 1; tmp[pos] = b'0' + (n % 10) as u8; n /= 10; }
    wr(&tmp[pos..]);
}

pub fn u64_to_buf(buf: &mut [u8], mut n: u64) -> usize {
    if n == 0 { buf[0] = b'0'; return 1; }
    let mut tmp = [0u8; 20];
    let mut pos = 20;
    while n > 0 { pos -= 1; tmp[pos] = b'0' + (n % 10) as u8; n /= 10; }
    let len = 20 - pos;
    buf[..len].copy_from_slice(&tmp[pos..]);
    len
}
