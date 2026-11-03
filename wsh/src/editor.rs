#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};
#[cfg(all(not(feature = "std"), test))]
use alloc::vec;

#[cfg(feature = "std")]
use std::{string::String, string::ToString, vec::Vec};
#[cfg(all(feature = "std", test))]
use std::vec;

use crate::highlight;
use crate::history::History;
use crate::platform;

pub struct LineEditor;

impl LineEditor {
    /// Считывает одну строку команды от пользователя в интерактивном режиме с правкой, историей и курсором
    pub fn read_line(prompt: &str, history: &mut History) -> Option<String> {
        print_raw(prompt);
        flush_raw();

        let _guard = platform::RawModeGuard::enter();

        let mut line_chars: Vec<char> = Vec::new();
        let mut cursor: usize = 0;
        let mut selection_anchor: Option<usize> = None;
        let mut consecutive_tabs: usize = 0;

        loop {
            let b = match platform::read_stdin_byte() {
                Some(byte) => byte,
                None => {
                    if line_chars.is_empty() {
                        return None;
                    }
                    print_raw("\n");
                    flush_raw();
                    return Some(line_chars.into_iter().collect());
                }
            };

            if b != b'\t' {
                consecutive_tabs = 0;
            }

            match b {
                // Enter
                b'\r' | b'\n' => {
                    let chars_after_cursor = line_chars.len().saturating_sub(cursor);
                    if chars_after_cursor > 0 {
                        print_ansi_motion('C', chars_after_cursor);
                    }
                    print_raw("\n");
                    flush_raw();
                    let res: String = line_chars.into_iter().collect();
                    return Some(res);
                }
                // Tab (Автодополнение команд и путей)
                b'\t' => {
                    selection_anchor = None;
                    consecutive_tabs += 1;
                    handle_tab_completion(prompt, &mut line_chars, &mut cursor, consecutive_tabs);
                }
                // Ctrl+C
                0x03 => {
                    print_raw("^C\n");
                    flush_raw();
                    return Some(String::from("\x03"));
                }
                // Ctrl+D (EOF при пустой строке, иначе удаление символа под курсором)
                0x04 => {
                    if line_chars.is_empty() {
                        print_raw("\n");
                        flush_raw();
                        return None;
                    } else if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                        cursor = delete_selection(&mut line_chars, sel);
                        selection_anchor = None;
                        redraw_line(prompt, &line_chars, cursor, None);
                    } else if delete_char(&mut line_chars, cursor) {
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+L (Clear screen)
                0x0C => {
                    print_raw("\x1b[2J\x1b[H");
                    redraw_line(prompt, &line_chars, cursor, get_selection_range(selection_anchor, cursor));
                }
                // Backspace (0x7F или 0x08)
                0x7F | 0x08 => {
                    if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                        cursor = delete_selection(&mut line_chars, sel);
                        selection_anchor = None;
                        redraw_line(prompt, &line_chars, cursor, None);
                    } else if cursor > 0 {
                        cursor -= 1;
                        line_chars.remove(cursor);
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+A (Home)
                0x01 => {
                    let had_sel = selection_anchor.is_some();
                    selection_anchor = None;
                    if cursor > 0 {
                        let count = cursor;
                        cursor = 0;
                        if had_sel {
                            redraw_line(prompt, &line_chars, cursor, None);
                        } else {
                            print_ansi_motion('D', count);
                        }
                    } else if had_sel {
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+E (End)
                0x05 => {
                    let had_sel = selection_anchor.is_some();
                    selection_anchor = None;
                    if cursor < line_chars.len() {
                        let count = line_chars.len() - cursor;
                        cursor = line_chars.len();
                        if had_sel {
                            redraw_line(prompt, &line_chars, cursor, None);
                        } else {
                            print_ansi_motion('C', count);
                        }
                    } else if had_sel {
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+B (Курсор влево)
                0x02 => {
                    move_cursor_left(prompt, &line_chars, &mut cursor, &mut selection_anchor);
                }
                // Ctrl+F (Курсор вправо)
                0x06 => {
                    move_cursor_right(prompt, &line_chars, &mut cursor, &mut selection_anchor);
                }
                // Ctrl+P (Предыдущая команда из истории)
                0x10 => {
                    selection_anchor = None;
                    if let Some(prev) = history.prev() {
                        replace_line(prompt, &mut line_chars, &mut cursor, prev);
                    }
                }
                // Ctrl+N (Следующая команда из истории)
                0x0E => {
                    selection_anchor = None;
                    if let Some(next) = history.next() {
                        replace_line(prompt, &mut line_chars, &mut cursor, next);
                    } else {
                        replace_line(prompt, &mut line_chars, &mut cursor, "");
                    }
                }
                // Ctrl+U (Очистить до начала строки в буфер вырезания)
                0x15 => {
                    selection_anchor = None;
                    if let Some(killed) = kill_to_start(&mut line_chars, &mut cursor) {
                        set_kill_ring(&killed);
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+K (Очистить до конца строки в буфер вырезания)
                0x0B => {
                    selection_anchor = None;
                    if let Some(killed) = kill_to_end(&mut line_chars, cursor) {
                        set_kill_ring(&killed);
                        print_raw("\x1b[K");
                        flush_raw();
                    }
                }
                // Ctrl+W (Удаление слова перед курсором)
                0x17 => {
                    selection_anchor = None;
                    if let Some(killed) = backward_kill_word(&mut line_chars, &mut cursor) {
                        set_kill_ring(&killed);
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+Y (Вставка из буфера вырезания / Yank)
                0x19 => {
                    let text = get_kill_ring();
                    if !text.is_empty() {
                        let sel = get_selection_range(selection_anchor, cursor);
                        paste_text(&mut line_chars, &mut cursor, sel, &text);
                        selection_anchor = None;
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+V (Вставка из буфера обмена / Paste)
                0x16 => {
                    let text = get_clipboard();
                    if !text.is_empty() {
                        let sel = get_selection_range(selection_anchor, cursor);
                        paste_text(&mut line_chars, &mut cursor, sel, &text);
                        selection_anchor = None;
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Ctrl+T (Поменять местами символы)
                0x14 => {
                    selection_anchor = None;
                    if line_chars.len() >= 2 {
                        transpose_chars(&mut line_chars, &mut cursor);
                        redraw_line(prompt, &line_chars, cursor, None);
                    }
                }
                // Escape-последовательности (стрелки, Home/End, Delete, CSI u Ctrl+Shift+C/V, Alt-хоткеи)
                0x1B => {
                    if let Some(next1) = platform::read_stdin_byte() {
                        match next1 {
                            b'[' => {
                                let action = parse_csi();
                                match action {
                                    CsiAction::Up => {
                                        selection_anchor = None;
                                        if let Some(prev) = history.prev() {
                                            replace_line(prompt, &mut line_chars, &mut cursor, prev);
                                        }
                                    }
                                    CsiAction::Down => {
                                        selection_anchor = None;
                                        if let Some(next) = history.next() {
                                            replace_line(prompt, &mut line_chars, &mut cursor, next);
                                        } else {
                                            replace_line(prompt, &mut line_chars, &mut cursor, "");
                                        }
                                    }
                                    CsiAction::Right => {
                                        move_cursor_right(prompt, &line_chars, &mut cursor, &mut selection_anchor);
                                    }
                                    CsiAction::Left => {
                                        move_cursor_left(prompt, &line_chars, &mut cursor, &mut selection_anchor);
                                    }
                                    CsiAction::Home => {
                                        let had_sel = selection_anchor.is_some();
                                        selection_anchor = None;
                                        if cursor > 0 {
                                            let count = cursor;
                                            cursor = 0;
                                            if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            } else {
                                                print_ansi_motion('D', count);
                                            }
                                        } else if had_sel {
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::End => {
                                        let had_sel = selection_anchor.is_some();
                                        selection_anchor = None;
                                        if cursor < line_chars.len() {
                                            let count = line_chars.len() - cursor;
                                            cursor = line_chars.len();
                                            if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            } else {
                                                print_ansi_motion('C', count);
                                            }
                                        } else if had_sel {
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::Delete => {
                                        if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                                            cursor = delete_selection(&mut line_chars, sel);
                                            selection_anchor = None;
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        } else if delete_char(&mut line_chars, cursor) {
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::WordBackward => {
                                        let had_sel = selection_anchor.is_some();
                                        selection_anchor = None;
                                        let new_cursor = backward_word(&line_chars, cursor);
                                        if new_cursor < cursor {
                                            let count = cursor - new_cursor;
                                            cursor = new_cursor;
                                            if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            } else {
                                                print_ansi_motion('D', count);
                                            }
                                        } else if had_sel {
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::WordForward => {
                                        let had_sel = selection_anchor.is_some();
                                        selection_anchor = None;
                                        let new_cursor = forward_word(&line_chars, cursor);
                                        if new_cursor > cursor {
                                            let count = new_cursor - cursor;
                                            cursor = new_cursor;
                                            if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            } else {
                                                print_ansi_motion('C', count);
                                            }
                                        } else if had_sel {
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::SelectLeft => {
                                        if cursor > 0 {
                                            if selection_anchor.is_none() {
                                                selection_anchor = Some(cursor);
                                            }
                                            cursor -= 1;
                                            redraw_line(prompt, &line_chars, cursor, get_selection_range(selection_anchor, cursor));
                                        }
                                    }
                                    CsiAction::SelectRight => {
                                        if cursor < line_chars.len() {
                                            if selection_anchor.is_none() {
                                                selection_anchor = Some(cursor);
                                            }
                                            cursor += 1;
                                            redraw_line(prompt, &line_chars, cursor, get_selection_range(selection_anchor, cursor));
                                        }
                                    }
                                    CsiAction::SelectHome => {
                                        if cursor > 0 {
                                            if selection_anchor.is_none() {
                                                selection_anchor = Some(cursor);
                                            }
                                            cursor = 0;
                                            redraw_line(prompt, &line_chars, cursor, get_selection_range(selection_anchor, cursor));
                                        }
                                    }
                                    CsiAction::SelectEnd => {
                                        if cursor < line_chars.len() {
                                            if selection_anchor.is_none() {
                                                selection_anchor = Some(cursor);
                                            }
                                            cursor = line_chars.len();
                                            redraw_line(prompt, &line_chars, cursor, get_selection_range(selection_anchor, cursor));
                                        }
                                    }
                                    CsiAction::Copy => {
                                        let sel = get_selection_range(selection_anchor, cursor);
                                        let text = copy_selection_or_line(&line_chars, sel);
                                        if !text.is_empty() {
                                            set_clipboard(&text);
                                        }
                                        selection_anchor = None;
                                        redraw_line(prompt, &line_chars, cursor, None);
                                    }
                                    CsiAction::Paste => {
                                        let text = get_clipboard();
                                        if !text.is_empty() {
                                            let sel = get_selection_range(selection_anchor, cursor);
                                            paste_text(&mut line_chars, &mut cursor, sel, &text);
                                            selection_anchor = None;
                                            redraw_line(prompt, &line_chars, cursor, None);
                                        }
                                    }
                                    CsiAction::Unknown => {}
                                }
                            }
                            // Старый стиль терминалов для Home / End (\x1bOH, \x1bOF)
                            b'O' => {
                                let had_sel = selection_anchor.is_some();
                                selection_anchor = None;
                                if let Some(code) = platform::read_stdin_byte() {
                                    match code {
                                        b'H' => {
                                            if cursor > 0 {
                                                let count = cursor;
                                                cursor = 0;
                                                if had_sel {
                                                    redraw_line(prompt, &line_chars, cursor, None);
                                                } else {
                                                    print_ansi_motion('D', count);
                                                }
                                            } else if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            }
                                        }
                                        b'F' => {
                                            if cursor < line_chars.len() {
                                                let count = line_chars.len() - cursor;
                                                cursor = line_chars.len();
                                                if had_sel {
                                                    redraw_line(prompt, &line_chars, cursor, None);
                                                } else {
                                                    print_ansi_motion('C', count);
                                                }
                                            } else if had_sel {
                                                redraw_line(prompt, &line_chars, cursor, None);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            // Alt+B (Слово назад)
                            b'b' | b'B' => {
                                let had_sel = selection_anchor.is_some();
                                selection_anchor = None;
                                let new_cursor = backward_word(&line_chars, cursor);
                                if new_cursor < cursor {
                                    let count = cursor - new_cursor;
                                    cursor = new_cursor;
                                    if had_sel {
                                        redraw_line(prompt, &line_chars, cursor, None);
                                    } else {
                                        print_ansi_motion('D', count);
                                    }
                                } else if had_sel {
                                    redraw_line(prompt, &line_chars, cursor, None);
                                }
                            }
                            // Alt+F (Слово вперед)
                            b'f' | b'F' => {
                                let had_sel = selection_anchor.is_some();
                                selection_anchor = None;
                                let new_cursor = forward_word(&line_chars, cursor);
                                if new_cursor > cursor {
                                    let count = new_cursor - cursor;
                                    cursor = new_cursor;
                                    if had_sel {
                                        redraw_line(prompt, &line_chars, cursor, None);
                                    } else {
                                        print_ansi_motion('C', count);
                                    }
                                } else if had_sel {
                                    redraw_line(prompt, &line_chars, cursor, None);
                                }
                            }
                            // Alt+D (Удалить слово вперед)
                            b'd' | b'D' => {
                                selection_anchor = None;
                                if let Some(killed) = forward_kill_word(&mut line_chars, cursor) {
                                    set_kill_ring(&killed);
                                    redraw_line(prompt, &line_chars, cursor, None);
                                }
                            }
                            // Alt+Backspace (Удалить слово назад)
                            0x7F | 0x08 => {
                                selection_anchor = None;
                                if let Some(killed) = backward_kill_word(&mut line_chars, &mut cursor) {
                                    set_kill_ring(&killed);
                                    redraw_line(prompt, &line_chars, cursor, None);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // Печатный ASCII (0x20..=0x7E)
                0x20..=0x7E => {
                    let ch = b as char;
                    if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                        cursor = delete_selection(&mut line_chars, sel);
                        selection_anchor = None;
                    }
                    insert_char(prompt, &mut line_chars, &mut cursor, ch);
                }
                // Кириллица и 2-байтовый UTF-8 (0xC0..=0xDF)
                0xC0..=0xDF => {
                    if let Some(b2) = platform::read_stdin_byte() {
                        let bytes = [b, b2];
                        if let Ok(s) = core::str::from_utf8(&bytes) {
                            if let Some(c) = s.chars().next() {
                                if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                                    cursor = delete_selection(&mut line_chars, sel);
                                    selection_anchor = None;
                                }
                                insert_char(prompt, &mut line_chars, &mut cursor, c);
                            }
                        }
                    }
                }
                // 3-байтовый UTF-8 (0xE0..=0xEF)
                0xE0..=0xEF => {
                    if let (Some(b2), Some(b3)) = (platform::read_stdin_byte(), platform::read_stdin_byte()) {
                        let bytes = [b, b2, b3];
                        if let Ok(s) = core::str::from_utf8(&bytes) {
                            if let Some(c) = s.chars().next() {
                                if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                                    cursor = delete_selection(&mut line_chars, sel);
                                    selection_anchor = None;
                                }
                                insert_char(prompt, &mut line_chars, &mut cursor, c);
                            }
                        }
                    }
                }
                // 4-байтовый UTF-8 (0xF0..=0xF7)
                0xF0..=0xF7 => {
                    if let (Some(b2), Some(b3), Some(b4)) = (
                        platform::read_stdin_byte(),
                        platform::read_stdin_byte(),
                        platform::read_stdin_byte(),
                    ) {
                        let bytes = [b, b2, b3, b4];
                        if let Ok(s) = core::str::from_utf8(&bytes) {
                            if let Some(c) = s.chars().next() {
                                if let Some(sel) = get_selection_range(selection_anchor, cursor) {
                                    cursor = delete_selection(&mut line_chars, sel);
                                    selection_anchor = None;
                                }
                                insert_char(prompt, &mut line_chars, &mut cursor, c);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TabCompletionAction {
    CompletedOne { suffix: String, kind: EntryKind },
    CompletedPrefix { suffix: String },
    ShowMatches { matches: Vec<(String, EntryKind)> },
    WaitNextTab,
    None,
}

pub fn compute_tab_completion(
    matches: &[(String, EntryKind)],
    prefix_to_complete: &str,
    tab_count: usize,
) -> TabCompletionAction {
    if matches.is_empty() {
        return TabCompletionAction::None;
    }

    if matches.len() == 1 {
        let (name, kind) = &matches[0];
        let suffix = if prefix_to_complete.len() <= name.len() {
            name[prefix_to_complete.len()..].to_string()
        } else {
            String::new()
        };
        return TabCompletionAction::CompletedOne {
            suffix,
            kind: *kind,
        };
    }

    let match_names: Vec<String> = matches.iter().map(|(n, _)| n.clone()).collect();
    let lcp = find_common_prefix(&match_names);

    if lcp.len() > prefix_to_complete.len() {
        let suffix = lcp[prefix_to_complete.len()..].to_string();
        TabCompletionAction::CompletedPrefix { suffix }
    } else if tab_count >= 2 {
        TabCompletionAction::ShowMatches {
            matches: matches.to_vec(),
        }
    } else {
        TabCompletionAction::WaitNextTab
    }
}

fn handle_tab_completion(prompt: &str, line: &mut Vec<char>, cursor: &mut usize, tab_count: usize) {
    let text_before: String = line[..*cursor].iter().collect();
    let token_start = text_before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    let current_token = &text_before[token_start..];

    let is_command = text_before[..token_start].trim().is_empty();

    let mut matches: Vec<(String, EntryKind)> = Vec::new();

    if is_command {
        for &cmd in crate::highlight::BUILTINS {
            if cmd.starts_with(current_token) {
                matches.push((cmd.to_string(), EntryKind::File));
            }
        }

        // Добавляем бинарники из /bin
        let bin_entries = read_dir_entries("/bin");
        for (name, kind) in bin_entries {
            if name.starts_with(current_token) && !matches.iter().any(|(m, _)| m == &name) {
                matches.push((name, kind));
            }
        }

        // Добавляем известные общесистемные команды как надежный fallback
        for &cmd in crate::highlight::COMMON_COMMANDS {
            if cmd.starts_with(current_token) && !matches.iter().any(|(m, _)| m == cmd) {
                matches.push((cmd.to_string(), EntryKind::File));
            }
        }
    } else {
        // Дополнение путей к файлам/директориям
        let (dir_path, file_prefix) = if let Some(last_slash) = current_token.rfind('/') {
            let d = if last_slash == 0 { "/" } else { &current_token[..last_slash] };
            let p = &current_token[last_slash + 1..];
            (d, p)
        } else {
            (".", current_token)
        };

        let entries = read_dir_entries(dir_path);
        for (name, kind) in entries {
            if (name == "." || name == "..") && !file_prefix.starts_with('.') {
                continue;
            }
            if name.starts_with(file_prefix) {
                matches.push((name, kind));
            }
        }
    }

    if matches.is_empty() {
        return;
    }

    let prefix_to_complete = if !is_command && current_token.contains('/') {
        let last_slash = current_token.rfind('/').unwrap();
        &current_token[last_slash + 1..]
    } else {
        current_token
    };

    match compute_tab_completion(&matches, prefix_to_complete, tab_count) {
        TabCompletionAction::CompletedOne { suffix, kind } => {
            for c in suffix.chars() {
                line.insert(*cursor, c);
                *cursor += 1;
            }
            if kind == EntryKind::Dir {
                line.insert(*cursor, '/');
                *cursor += 1;
            } else {
                line.insert(*cursor, ' ');
                *cursor += 1;
            }
            redraw_line(prompt, line, *cursor, None);
        }
        TabCompletionAction::CompletedPrefix { suffix } => {
            for c in suffix.chars() {
                line.insert(*cursor, c);
                *cursor += 1;
            }
            redraw_line(prompt, line, *cursor, None);
        }
        TabCompletionAction::ShowMatches { matches } => {
            print_raw("\n");
            for (name, kind) in &matches {
                match kind {
                    EntryKind::Dir => {
                        print_raw("\x1b[1;34m");
                        print_raw(name);
                        print_raw("/\x1b[0m  ");
                    }
                    EntryKind::Symlink => {
                        print_raw("\x1b[1;36m");
                        print_raw(name);
                        print_raw("@\x1b[0m  ");
                    }
                    EntryKind::File => {
                        print_raw(name);
                        print_raw("  ");
                    }
                }
            }
            print_raw("\n");
            redraw_line(prompt, line, *cursor, None);
        }
        TabCompletionAction::WaitNextTab | TabCompletionAction::None => {}
    }
}

pub fn find_common_prefix(words: &[String]) -> String {
    if words.is_empty() {
        return String::new();
    }
    let first = &words[0];
    let mut prefix_len = 0;
    for (i, c) in first.chars().enumerate() {
        if words.iter().all(|w| w.chars().nth(i) == Some(c)) {
            prefix_len += c.len_utf8();
        } else {
            break;
        }
    }
    first[..prefix_len].to_string()
}

fn read_dir_entries(dir_path: &str) -> Vec<(String, EntryKind)> {
    let mut entries = Vec::new();

    #[cfg(feature = "std")]
    {
        if let Ok(read_dir) = std::fs::read_dir(dir_path) {
            for entry in read_dir.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let kind = if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            EntryKind::Dir
                        } else if ft.is_symlink() {
                            EntryKind::Symlink
                        } else {
                            EntryKind::File
                        }
                    } else {
                        EntryKind::File
                    };
                    entries.push((name, kind));
                }
            }
        }
    }

    #[cfg(not(feature = "std"))]
    {
        let mut null_term = [0u8; 256];
        let bytes = dir_path.as_bytes();
        if bytes.len() < null_term.len() {
            null_term[..bytes.len()].copy_from_slice(bytes);
            null_term[bytes.len()] = 0;

            let fd = libwunix::open(&null_term[..bytes.len() + 1], 0, 0);
            if fd >= 0 {
                let mut buf = [0u8; 4096];
                loop {
                    let n = libwunix::getdents64(fd as u64, &mut buf);
                    if n <= 0 {
                        break;
                    }
                    let total = (n as usize).min(buf.len());
                    let mut pos = 0;
                    while pos + 19 <= total {
                        let reclen = u16::from_ne_bytes([buf[pos + 16], buf[pos + 17]]) as usize;
                        if reclen == 0 || pos + reclen > total {
                            break;
                        }
                        let dtype = buf[pos + 18];
                        let kind = match dtype {
                            4 => EntryKind::Dir,
                            10 => EntryKind::Symlink,
                            _ => EntryKind::File,
                        };

                        let name_start = pos + 19;
                        let mut name_end = name_start;
                        let max_end = pos + reclen;
                        while name_end < max_end && buf[name_end] != 0 {
                            name_end += 1;
                        }
                        if name_end > name_start {
                            if let Ok(name_str) = core::str::from_utf8(&buf[name_start..name_end]) {
                                entries.push((String::from(name_str), kind));
                            }
                        }
                        pos += reclen;
                    }
                }
                let _ = libwunix::close(fd as u64);
            }
        }
    }

    entries
}

fn insert_char(prompt: &str, line: &mut Vec<char>, cursor: &mut usize, c: char) {
    line.insert(*cursor, c);
    *cursor += 1;
    redraw_line(prompt, line, *cursor, None);
}

fn replace_line(prompt: &str, line: &mut Vec<char>, cursor: &mut usize, text: &str) {
    line.clear();
    for c in text.chars() {
        line.push(c);
    }
    *cursor = line.len();
    redraw_line(prompt, line, *cursor, None);
}

pub fn move_cursor_left(
    prompt: &str,
    line: &[char],
    cursor: &mut usize,
    selection_anchor: &mut Option<usize>,
) {
    let had_sel = selection_anchor.is_some();
    *selection_anchor = None;
    if *cursor > 0 {
        *cursor -= 1;
        if had_sel {
            redraw_line(prompt, line, *cursor, None);
        } else {
            print_raw("\x1b[D");
            flush_raw();
        }
    } else if had_sel {
        redraw_line(prompt, line, *cursor, None);
    }
}

pub fn move_cursor_right(
    prompt: &str,
    line: &[char],
    cursor: &mut usize,
    selection_anchor: &mut Option<usize>,
) {
    let had_sel = selection_anchor.is_some();
    *selection_anchor = None;
    if *cursor < line.len() {
        *cursor += 1;
        if had_sel {
            redraw_line(prompt, line, *cursor, None);
        } else {
            print_raw("\x1b[C");
            flush_raw();
        }
    } else if had_sel {
        redraw_line(prompt, line, *cursor, None);
    }
}

fn redraw_line(
    prompt: &str,
    line: &[char],
    cursor: usize,
    selection: Option<(usize, usize)>,
) {
    let mut buf = String::with_capacity(64 + prompt.len() + line.len() * 8);
    // Скрываем аппаратный курсор на время отрисовки, чтобы терминал не моргал и не дергал курсор по экрану
    buf.push_str("\x1b[?25l\r\x1b[K");
    buf.push_str(prompt);

    match selection {
        Some((start, end)) if start < end => {
            let s = start.min(line.len());
            let e = end.min(line.len());

            // До выделения — с подсветкой синтаксиса
            let highlighted = highlight::highlight_line(line);
            // Для выделения используем инверсию, поэтому рисуем по частям без подсветки
            for &c in &line[..s] {
                buf.push(c);
            }
            // Включаем инверсию цветов (видеовыделение)
            buf.push_str("\x1b[7m");
            for &c in &line[s..e] {
                buf.push(c);
            }
            // Сбрасываем инверсию цветов
            buf.push_str("\x1b[27m");
            for &c in &line[e..] {
                buf.push(c);
            }
            // highlighted нужен только для определения что вывести, но при выделении используем plain
            let _ = highlighted;
        }
        _ => {
            // Обычный режим — синтаксическая подсветка в реальном времени
            let highlighted = highlight::highlight_line(line);
            buf.push_str(&highlighted);
        }
    }

    // Отводим курсор назад с учётом позиции в строке.
    // Количество видимых символов после позиции курсора = длина хвоста.
    let chars_after_cursor = line.len().saturating_sub(cursor);
    if chars_after_cursor > 0 {
        let _ = core::fmt::write(&mut buf, format_args!("\x1b[{}D", chars_after_cursor));
    }
    buf.push_str("\x1b[?25h");
    print_raw(&buf);
    flush_raw();
}

fn print_raw(s: &str) {
    #[cfg(feature = "std")]
    {
        use std::io::Write;
        let _ = std::io::stdout().write_all(s.as_bytes());
    }
    #[cfg(not(feature = "std"))]
    {
        libwunix::write(libwunix::STDOUT_FILENO, s.as_bytes());
    }
}

fn print_ansi_motion(dir: char, count: usize) {
    if count == 0 {
        return;
    }
    let mut buf = String::with_capacity(16);
    let _ = core::fmt::write(&mut buf, format_args!("\x1b[{}{}", count, dir));
    print_raw(&buf);
    flush_raw();
}

fn flush_raw() {
    #[cfg(feature = "std")]
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
}

struct EditorMutex<T> {
    lock: core::sync::atomic::AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for EditorMutex<T> {}

impl<T> EditorMutex<T> {
    const fn new(data: T) -> Self {
        Self {
            lock: core::sync::atomic::AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }

    fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.lock.swap(true, core::sync::atomic::Ordering::Acquire) {
            core::hint::spin_loop();
        }
        let res = f(unsafe { &mut *self.data.get() });
        self.lock.store(false, core::sync::atomic::Ordering::Release);
        res
    }
}

static KILL_RING: EditorMutex<Option<String>> = EditorMutex::new(None);

pub fn set_kill_ring(text: &str) {
    if !text.is_empty() {
        KILL_RING.lock(|slot| {
            *slot = Some(String::from(text));
        });
    }
}

pub fn get_kill_ring() -> String {
    KILL_RING.lock(|slot| slot.clone().unwrap_or_default())
}

pub fn backward_kill_word(line: &mut Vec<char>, cursor: &mut usize) -> Option<String> {
    if *cursor == 0 || line.is_empty() {
        return None;
    }
    let mut new_cursor = *cursor;
    while new_cursor > 0 && line[new_cursor - 1].is_whitespace() {
        new_cursor -= 1;
    }
    while new_cursor > 0 && !line[new_cursor - 1].is_whitespace() {
        new_cursor -= 1;
    }
    if new_cursor < *cursor {
        let killed: String = line.drain(new_cursor..*cursor).collect();
        *cursor = new_cursor;
        Some(killed)
    } else {
        None
    }
}

pub fn forward_kill_word(line: &mut Vec<char>, cursor: usize) -> Option<String> {
    if cursor >= line.len() {
        return None;
    }
    let mut end = cursor;
    while end < line.len() && line[end].is_whitespace() {
        end += 1;
    }
    while end < line.len() && !line[end].is_whitespace() {
        end += 1;
    }
    if end > cursor {
        let killed: String = line.drain(cursor..end).collect();
        Some(killed)
    } else {
        None
    }
}

pub fn backward_word(line: &[char], cursor: usize) -> usize {
    if cursor == 0 {
        return 0;
    }
    let mut pos = cursor;
    while pos > 0 && line[pos - 1].is_whitespace() {
        pos -= 1;
    }
    while pos > 0 && !line[pos - 1].is_whitespace() {
        pos -= 1;
    }
    pos
}

pub fn forward_word(line: &[char], cursor: usize) -> usize {
    let mut pos = cursor;
    while pos < line.len() && line[pos].is_whitespace() {
        pos += 1;
    }
    while pos < line.len() && !line[pos].is_whitespace() {
        pos += 1;
    }
    pos
}

pub fn transpose_chars(line: &mut Vec<char>, cursor: &mut usize) {
    if line.len() < 2 {
        return;
    }
    let idx = if *cursor == line.len() {
        *cursor - 1
    } else if *cursor > 0 {
        *cursor
    } else {
        1
    };
    line.swap(idx - 1, idx);
    if *cursor < line.len() {
        *cursor += 1;
    }
}

pub fn kill_to_start(line: &mut Vec<char>, cursor: &mut usize) -> Option<String> {
    if *cursor == 0 {
        return None;
    }
    let killed: String = line.drain(..*cursor).collect();
    *cursor = 0;
    Some(killed)
}

pub fn kill_to_end(line: &mut Vec<char>, cursor: usize) -> Option<String> {
    if cursor >= line.len() {
        return None;
    }
    let killed: String = line.drain(cursor..).collect();
    Some(killed)
}

pub fn yank(line: &mut Vec<char>, cursor: &mut usize, text: &str) {
    for c in text.chars() {
        line.insert(*cursor, c);
        *cursor += 1;
    }
}

pub fn delete_char(line: &mut Vec<char>, cursor: usize) -> bool {
    if cursor < line.len() {
        line.remove(cursor);
        true
    } else {
        false
    }
}

static CLIPBOARD: EditorMutex<Option<String>> = EditorMutex::new(None);

pub fn set_clipboard(text: &str) {
    if !text.is_empty() {
        CLIPBOARD.lock(|slot| {
            *slot = Some(String::from(text));
        });
    }
}

pub fn get_clipboard() -> String {
    let cb = CLIPBOARD.lock(|slot| slot.clone().unwrap_or_default());
    if cb.is_empty() {
        get_kill_ring()
    } else {
        cb
    }
}

pub fn get_selection_range(anchor: Option<usize>, cursor: usize) -> Option<(usize, usize)> {
    let a = anchor?;
    if a == cursor {
        None
    } else if a < cursor {
        Some((a, cursor))
    } else {
        Some((cursor, a))
    }
}

pub fn copy_selection_or_line(line: &[char], selection: Option<(usize, usize)>) -> String {
    if let Some((start, end)) = selection {
        let s = start.min(line.len());
        let e = end.min(line.len());
        if s < e {
            return line[s..e].iter().collect();
        }
    }
    String::new()
}

pub fn delete_selection(line: &mut Vec<char>, selection: (usize, usize)) -> usize {
    let (start, end) = selection;
    let s = start.min(line.len());
    let e = end.min(line.len());
    if s < e {
        line.drain(s..e);
    }
    s
}

pub fn paste_text(
    line: &mut Vec<char>,
    cursor: &mut usize,
    selection: Option<(usize, usize)>,
    text: &str,
) {
    if let Some(sel) = selection {
        *cursor = delete_selection(line, sel);
    }
    for c in text.chars() {
        if c == '\r' || c == '\n' {
            continue;
        }
        line.insert(*cursor, c);
        *cursor += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsiAction {
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Delete,
    WordBackward,
    WordForward,
    SelectLeft,
    SelectRight,
    SelectHome,
    SelectEnd,
    Copy,
    Paste,
    Unknown,
}

pub fn decode_csi(final_byte: u8, p1: usize, p2: usize, p3: usize) -> CsiAction {
    match final_byte {
        b'A' => CsiAction::Up,
        b'B' => CsiAction::Down,
        b'C' => {
            if p2 == 5 || p1 == 5 {
                CsiAction::WordForward
            } else if p2 == 2 || p1 == 2 {
                CsiAction::SelectRight
            } else {
                CsiAction::Right
            }
        }
        b'D' => {
            if p2 == 5 || p1 == 5 {
                CsiAction::WordBackward
            } else if p2 == 2 || p1 == 2 {
                CsiAction::SelectLeft
            } else {
                CsiAction::Left
            }
        }
        b'H' => {
            if p2 == 2 || p1 == 2 {
                CsiAction::SelectHome
            } else {
                CsiAction::Home
            }
        }
        b'F' => {
            if p2 == 2 || p1 == 2 {
                CsiAction::SelectEnd
            } else {
                CsiAction::End
            }
        }
        b'~' => match p1 {
            1 | 7 => CsiAction::Home,
            4 | 8 => CsiAction::End,
            3 => CsiAction::Delete,
            27 => {
                if p2 == 6 {
                    if p3 == 99 || p3 == 67 {
                        CsiAction::Copy
                    } else if p3 == 118 || p3 == 86 {
                        CsiAction::Paste
                    } else {
                        CsiAction::Unknown
                    }
                } else {
                    CsiAction::Unknown
                }
            }
            _ => CsiAction::Unknown,
        },
        // CSI u: \x1b[99;6u (Ctrl+Shift+C), \x1b[118;6u (Ctrl+Shift+V)
        b'u' => {
            if (p1 == 99 || p1 == 67) && p2 == 6 {
                CsiAction::Copy
            } else if (p1 == 118 || p1 == 86) && (p2 == 6 || p2 == 5) {
                CsiAction::Paste
            } else {
                CsiAction::Unknown
            }
        }
        _ => CsiAction::Unknown,
    }
}

fn parse_csi() -> CsiAction {
    let mut params = [0usize; 4];
    let mut param_idx = 0;

    for _ in 0..16 {
        let b = match platform::read_stdin_byte() {
            Some(byte) => byte,
            None => return CsiAction::Unknown,
        };

        match b {
            b'0'..=b'9' => {
                if param_idx < params.len() {
                    params[param_idx] = params[param_idx]
                        .saturating_mul(10)
                        .saturating_add((b - b'0') as usize);
                }
            }
            b';' => {
                if param_idx + 1 < params.len() {
                    param_idx += 1;
                }
            }
            0x40..=0x7E => {
                let p1 = params[0];
                let p2 = params[1];
                let p3 = params[2];
                return decode_csi(b, p1, p2, p3);
            }
            _ => {
                return CsiAction::Unknown;
            }
        }
    }
    CsiAction::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_common_prefix() {
        let v1 = vec!["echo".to_string(), "exit".to_string()];
        assert_eq!(find_common_prefix(&v1), "e");

        let v2 = vec!["clear".to_string(), "clean".to_string(), "client".to_string()];
        assert_eq!(find_common_prefix(&v2), "cl");

        let v3 = vec!["uname".to_string(), "uname".to_string()];
        assert_eq!(find_common_prefix(&v3), "uname");

        let empty: Vec<String> = Vec::new();
        assert_eq!(find_common_prefix(&empty), "");
    }

    #[test]
    fn test_backward_kill_word() {
        let mut line: Vec<char> = "echo hello world".chars().collect();
        let mut cursor = line.len();
        let killed = backward_kill_word(&mut line, &mut cursor);
        assert_eq!(killed, Some("world".to_string()));
        assert_eq!(line.iter().collect::<String>(), "echo hello ");
        assert_eq!(cursor, 11);

        let mut line2: Vec<char> = "cargo test   ".chars().collect();
        let mut cursor2 = line2.len();
        let killed2 = backward_kill_word(&mut line2, &mut cursor2);
        assert_eq!(killed2, Some("test   ".to_string()));
        assert_eq!(line2.iter().collect::<String>(), "cargo ");
        assert_eq!(cursor2, 6);

        let mut line3: Vec<char> = "abc".chars().collect();
        let mut cursor3 = 0;
        assert_eq!(backward_kill_word(&mut line3, &mut cursor3), None);
        assert_eq!(line3.iter().collect::<String>(), "abc");
    }

    #[test]
    fn test_forward_kill_word() {
        let mut line: Vec<char> = "echo hello world".chars().collect();
        let killed = forward_kill_word(&mut line, 0);
        assert_eq!(killed, Some("echo".to_string()));
        assert_eq!(line.iter().collect::<String>(), " hello world");

        let mut line2: Vec<char> = "   hello".chars().collect();
        let killed2 = forward_kill_word(&mut line2, 0);
        assert_eq!(killed2, Some("   hello".to_string()));
        assert_eq!(line2.iter().collect::<String>(), "");
    }

    #[test]
    fn test_word_navigation() {
        let line: Vec<char> = "echo  hello   world".chars().collect();
        assert_eq!(backward_word(&line, 19), 14);
        assert_eq!(backward_word(&line, 14), 6);
        assert_eq!(backward_word(&line, 6), 0);
        assert_eq!(backward_word(&line, 0), 0);

        assert_eq!(forward_word(&line, 0), 4);
        assert_eq!(forward_word(&line, 4), 11);
        assert_eq!(forward_word(&line, 11), 19);
        assert_eq!(forward_word(&line, 19), 19);
    }

    #[test]
    fn test_transpose_chars() {
        let mut line: Vec<char> = "ab".chars().collect();
        let mut cursor = 2;
        transpose_chars(&mut line, &mut cursor);
        assert_eq!(line.iter().collect::<String>(), "ba");
        assert_eq!(cursor, 2);

        let mut line2: Vec<char> = "wunix".chars().collect();
        let mut cursor2 = 2;
        transpose_chars(&mut line2, &mut cursor2);
        assert_eq!(line2.iter().collect::<String>(), "wnuix");
        assert_eq!(cursor2, 3);
    }

    #[test]
    fn test_kill_to_start_and_end() {
        let mut line: Vec<char> = "echo hello world".chars().collect();
        let mut cursor = 10;
        let killed = kill_to_start(&mut line, &mut cursor);
        assert_eq!(killed, Some("echo hello".to_string()));
        assert_eq!(line.iter().collect::<String>(), " world");
        assert_eq!(cursor, 0);

        let mut line2: Vec<char> = "echo hello world".chars().collect();
        let killed2 = kill_to_end(&mut line2, 10);
        assert_eq!(killed2, Some(" world".to_string()));
        assert_eq!(line2.iter().collect::<String>(), "echo hello");
    }

    #[test]
    fn test_yank_and_kill_ring() {
        set_kill_ring("inserted_text");
        assert_eq!(get_kill_ring(), "inserted_text");

        let mut line: Vec<char> = "hello !".chars().collect();
        let mut cursor = 6;
        yank(&mut line, &mut cursor, &get_kill_ring());
        assert_eq!(line.iter().collect::<String>(), "hello inserted_text!");
        assert_eq!(cursor, 19);
    }

    #[test]
    fn test_delete_char() {
        let mut line: Vec<char> = "test".chars().collect();
        assert!(delete_char(&mut line, 1));
        assert_eq!(line.iter().collect::<String>(), "tst");

        assert!(!delete_char(&mut line, 3));
    }

    #[test]
    fn test_decode_csi() {
        // Delete
        assert_eq!(decode_csi(b'~', 3, 0, 0), CsiAction::Delete);

        // CSI u: Ctrl+Shift+C и Ctrl+Shift+V
        assert_eq!(decode_csi(b'u', 99, 6, 0), CsiAction::Copy);
        assert_eq!(decode_csi(b'u', 67, 6, 0), CsiAction::Copy);
        assert_eq!(decode_csi(b'u', 118, 6, 0), CsiAction::Paste);
        assert_eq!(decode_csi(b'u', 86, 6, 0), CsiAction::Paste);

        // xterm: modifyOtherKeys \x1b[27;6;key~
        assert_eq!(decode_csi(b'~', 27, 6, 99), CsiAction::Copy);
        assert_eq!(decode_csi(b'~', 27, 6, 118), CsiAction::Paste);

        // Ctrl + стрелки (переход по словам)
        assert_eq!(decode_csi(b'D', 1, 5, 0), CsiAction::WordBackward);
        assert_eq!(decode_csi(b'C', 1, 5, 0), CsiAction::WordForward);

        // Shift + стрелки / Home / End (выделение текста)
        assert_eq!(decode_csi(b'D', 1, 2, 0), CsiAction::SelectLeft);
        assert_eq!(decode_csi(b'C', 1, 2, 0), CsiAction::SelectRight);
        assert_eq!(decode_csi(b'H', 1, 2, 0), CsiAction::SelectHome);
        assert_eq!(decode_csi(b'F', 1, 2, 0), CsiAction::SelectEnd);

        // Обычные стрелки
        assert_eq!(decode_csi(b'A', 0, 0, 0), CsiAction::Up);
        assert_eq!(decode_csi(b'B', 0, 0, 0), CsiAction::Down);
        assert_eq!(decode_csi(b'C', 0, 0, 0), CsiAction::Right);
        assert_eq!(decode_csi(b'D', 0, 0, 0), CsiAction::Left);
        assert_eq!(decode_csi(b'H', 0, 0, 0), CsiAction::Home);
        assert_eq!(decode_csi(b'F', 0, 0, 0), CsiAction::End);

        // Неизвестный ввод
        assert_eq!(decode_csi(b'z', 0, 0, 0), CsiAction::Unknown);
    }

    #[test]
    fn test_get_selection_range() {
        assert_eq!(get_selection_range(None, 5), None);
        assert_eq!(get_selection_range(Some(5), 5), None);
        assert_eq!(get_selection_range(Some(2), 7), Some((2, 7)));
        assert_eq!(get_selection_range(Some(7), 2), Some((2, 7)));
    }

    #[test]
    fn test_copy_selection_or_line() {
        let line: Vec<char> = "hello world".chars().collect();
        assert_eq!(copy_selection_or_line(&line, Some((0, 5))), "hello");
        assert_eq!(copy_selection_or_line(&line, Some((6, 11))), "world");
        assert_eq!(copy_selection_or_line(&line, None), "");
    }

    #[test]
    fn test_delete_selection() {
        let mut line: Vec<char> = "hello world".chars().collect();
        let cursor = delete_selection(&mut line, (0, 6));
        assert_eq!(line.iter().collect::<String>(), "world");
        assert_eq!(cursor, 0);

        let mut line2: Vec<char> = "echo test command".chars().collect();
        let cursor2 = delete_selection(&mut line2, (5, 9));
        assert_eq!(line2.iter().collect::<String>(), "echo  command");
        assert_eq!(cursor2, 5);
    }

    #[test]
    fn test_paste_text() {
        let mut line: Vec<char> = "hello !".chars().collect();
        let mut cursor = 6;
        paste_text(&mut line, &mut cursor, None, "world");
        assert_eq!(line.iter().collect::<String>(), "hello world!");
        assert_eq!(cursor, 11);

        // Вставка с заменой выделения
        let mut line2: Vec<char> = "echo old_arg end".chars().collect();
        let mut cursor2 = 12;
        paste_text(&mut line2, &mut cursor2, Some((5, 12)), "new_arg");
        assert_eq!(line2.iter().collect::<String>(), "echo new_arg end");
        assert_eq!(cursor2, 12);
    }

    #[test]
    fn test_clipboard_and_kill_ring_fallback() {
        set_clipboard("copied_text");
        assert_eq!(get_clipboard(), "copied_text");
    }

    #[test]
    fn test_move_cursor_bounds() {
        let prompt = "> ";
        let empty_line: Vec<char> = Vec::new();
        let mut cursor = 0;
        let mut selection = None;

        // Попытка выйти влево на пустой строке не должна сдвигать курсор
        move_cursor_left(prompt, &empty_line, &mut cursor, &mut selection);
        assert_eq!(cursor, 0);

        // Попытка выйти вправо на пустой строке не должна сдвигать курсор
        move_cursor_right(prompt, &empty_line, &mut cursor, &mut selection);
        assert_eq!(cursor, 0);

        // Непустая строка
        let line: Vec<char> = "abc".chars().collect();
        let mut cursor = 3;
        let mut selection = None;

        // Вправо на конце строки не должно сдвигать курсор
        move_cursor_right(prompt, &line, &mut cursor, &mut selection);
        assert_eq!(cursor, 3);

        // Движение влево
        move_cursor_left(prompt, &line, &mut cursor, &mut selection);
        assert_eq!(cursor, 2);
        move_cursor_left(prompt, &line, &mut cursor, &mut selection);
        assert_eq!(cursor, 1);
        move_cursor_left(prompt, &line, &mut cursor, &mut selection);
        assert_eq!(cursor, 0);

        // Влево в начале строки (на приглашение) не должно сдвигать курсор
        move_cursor_left(prompt, &line, &mut cursor, &mut selection);
        assert_eq!(cursor, 0);

        // Сброс выделения при границах
        let mut selection_at_zero = Some(0);
        move_cursor_left(prompt, &line, &mut cursor, &mut selection_at_zero);
        assert_eq!(cursor, 0);
        assert_eq!(selection_at_zero, None);

        cursor = 3;
        let mut selection_at_end = Some(3);
        move_cursor_right(prompt, &line, &mut cursor, &mut selection_at_end);
        assert_eq!(cursor, 3);
        assert_eq!(selection_at_end, None);
    }

    #[test]
    fn test_compute_tab_completion_single() {
        let matches = vec![("echo".to_string(), EntryKind::File)];
        let action = compute_tab_completion(&matches, "ec", 1);
        assert_eq!(
            action,
            TabCompletionAction::CompletedOne {
                suffix: "ho".to_string(),
                kind: EntryKind::File,
            }
        );
    }

    #[test]
    fn test_compute_tab_completion_lcp_first_tab() {
        let matches = vec![
            ("clear".to_string(), EntryKind::File),
            ("clean".to_string(), EntryKind::File),
        ];
        // Префикс "c" дополняется до "clea" на первом табе
        let action = compute_tab_completion(&matches, "c", 1);
        assert_eq!(
            action,
            TabCompletionAction::CompletedPrefix {
                suffix: "lea".to_string(),
            }
        );
    }

    #[test]
    fn test_compute_tab_completion_double_tab_requirement() {
        let matches = vec![
            ("clear".to_string(), EntryKind::File),
            ("clean".to_string(), EntryKind::File),
        ];
        // Уже достигли общего префикса "clea".
        // 1-й таб: не показывает подсказки, ждет второго таба!
        let action1 = compute_tab_completion(&matches, "clea", 1);
        assert_eq!(action1, TabCompletionAction::WaitNextTab);

        // 2-й таб: выводит подсказки!
        let action2 = compute_tab_completion(&matches, "clea", 2);
        assert_eq!(
            action2,
            TabCompletionAction::ShowMatches {
                matches: matches.clone(),
            }
        );
    }

    #[test]
    fn test_tab_completion_system_commands() {
        let mut line: Vec<char> = "shut".chars().collect();
        let mut cursor = line.len();
        handle_tab_completion("", &mut line, &mut cursor, 1);
        let completed: String = line.iter().collect();
        assert_eq!(completed, "shutdown ");

        let mut line2: Vec<char> = "reb".chars().collect();
        let mut cursor2 = line2.len();
        handle_tab_completion("", &mut line2, &mut cursor2, 1);
        let completed2: String = line2.iter().collect();
        assert_eq!(completed2, "reboot ");
    }
}
