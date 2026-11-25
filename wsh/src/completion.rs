extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Builtin,
    Command,
    File,
    Directory,
    Symlink,
    Variable,
    Subcommand,
    Option,
    Argument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub text: String,
    pub display_name: String,
    pub kind: CompletionKind,
}

impl CompletionItem {
    pub fn is_no_space(&self) -> bool {
        self.kind == CompletionKind::Directory || self.text.ends_with('=')
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionInput {
    pub token_start: usize,
    pub current_token: String,
    pub prev_words: Vec<String>,
}

/// Разбирает символы до курсора в текущий токен и список предшествующих слов активного сегмента команды.
/// Все индексы строго символьные (`char`), чтобы кириллица и UTF-8 не сдвигали позицию в `Vec<char>`.
pub fn parse_completion_input(chars_before: &[char]) -> CompletionInput {
    let mut prev_words: Vec<String> = Vec::new();
    let mut cur_word = String::new();
    let mut cur_start: Option<usize> = None;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut i = 0;

    while i < chars_before.len() {
        let c = chars_before[i];

        if escaped {
            if cur_start.is_none() {
                cur_start = Some(i.saturating_sub(1));
            }
            cur_word.push(c);
            escaped = false;
            i += 1;
            continue;
        }

        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if q == '"' && c == '\\' {
                escaped = true;
            } else {
                cur_word.push(c);
            }
            i += 1;
            continue;
        }

        if c == '\\' {
            if cur_start.is_none() {
                cur_start = Some(i);
            }
            escaped = true;
            i += 1;
            continue;
        }

        if c == '\'' || c == '"' {
            if cur_start.is_none() {
                cur_start = Some(i);
            }
            quote = Some(c);
            i += 1;
            continue;
        }

        // Разделители команд и конвейеры начинают новый командный сегмент
        if c == ';' || c == '|' || c == '&' {
            prev_words.clear();
            cur_word.clear();
            cur_start = None;
            if (c == '|' || c == '&') && i + 1 < chars_before.len() && chars_before[i + 1] == c {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        if c.is_whitespace() {
            if cur_start.is_some() {
                prev_words.push(cur_word);
                cur_word = String::new();
                cur_start = None;
            }
            i += 1;
            continue;
        }

        if cur_start.is_none() {
            cur_start = Some(i);
        }
        cur_word.push(c);
        i += 1;
    }

    CompletionInput {
        token_start: cur_start.unwrap_or(chars_before.len()),
        current_token: cur_word,
        prev_words,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    Command,
    Variable,
    Path,
}

/// Анализирует введенный текст до курсора и определяет контекст дополнения
pub fn detect_completion_context(text_before: &str, current_token: &str) -> CompletionContext {
    if current_token.starts_with('$') {
        return CompletionContext::Variable;
    }

    let token_start = text_before.rfind(' ').map(|i| i + 1).unwrap_or(0);
    let before_token = text_before[..token_start].trim();

    if before_token.is_empty() {
        return CompletionContext::Command;
    }

    // Ищем последнее слово перед текущим токеном
    let last_word = before_token.split_whitespace().last().unwrap_or("");

    // Разделители команд и конвейеры начинают новую команду
    if last_word == "|" || last_word == "||" || last_word == "&&" || last_word == ";" || last_word == "&" {
        return CompletionContext::Command;
    }

    // Утилиты-обертки запускают другую команду
    if matches!(
        last_word,
        "sudo" | "time" | "exec" | "which" | "type" | "man" | "help"
    ) {
        return CompletionContext::Command;
    }

    CompletionContext::Path
}

/// Находит наибольший общий префикс среди набора строк
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

/// Состояние интерактивного меню и навигации по вариантам автодополнения
#[derive(Default)]
pub struct CompletionState {
    pub is_active: bool,
    pub menu_rows: usize,
    pub num_cols: usize,
    pub scroll_row: usize,
    pub token_start: usize,
    pub original_token: String,
    pub candidates: Vec<CompletionItem>,
    pub current_index: Option<usize>,
}

impl CompletionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.is_active = false;
        self.menu_rows = 0;
        self.num_cols = 1;
        self.scroll_row = 0;
        self.token_start = 0;
        self.original_token.clear();
        self.candidates.clear();
        self.current_index = None;
    }

    pub fn current_item(&self) -> Option<&CompletionItem> {
        self.current_index.and_then(|idx| self.candidates.get(idx))
    }

    pub fn cycle_next(&mut self) -> Option<&CompletionItem> {
        if self.candidates.is_empty() {
            return None;
        }
        let next = match self.current_index {
            Some(idx) => (idx + 1) % self.candidates.len(),
            None => 0,
        };
        self.current_index = Some(next);
        Some(&self.candidates[next])
    }

    pub fn cycle_prev(&mut self) -> Option<&CompletionItem> {
        if self.candidates.is_empty() {
            return None;
        }
        let prev = match self.current_index {
            Some(idx) => (idx + self.candidates.len() - 1) % self.candidates.len(),
            None => self.candidates.len().saturating_sub(1),
        };
        self.current_index = Some(prev);
        Some(&self.candidates[prev])
    }

    pub fn move_right(&mut self) -> Option<&CompletionItem> {
        self.cycle_next()
    }

    pub fn move_left(&mut self) -> Option<&CompletionItem> {
        self.cycle_prev()
    }

    pub fn move_down(&mut self) -> Option<&CompletionItem> {
        if self.candidates.is_empty() {
            return None;
        }
        let cols = self.num_cols.max(1);
        let len = self.candidates.len();
        let next = match self.current_index {
            None => 0,
            Some(idx) => {
                if idx + cols < len {
                    idx + cols
                } else {
                    idx % cols
                }
            }
        };
        self.current_index = Some(next);
        Some(&self.candidates[next])
    }

    pub fn move_up(&mut self) -> Option<&CompletionItem> {
        if self.candidates.is_empty() {
            return None;
        }
        let cols = self.num_cols.max(1);
        let len = self.candidates.len();
        let prev = match self.current_index {
            None => len.saturating_sub(1),
            Some(idx) => {
                if idx >= cols {
                    idx - cols
                } else {
                    let col = idx % cols;
                    let mut bottom = col;
                    while bottom + cols < len {
                        bottom += cols;
                    }
                    bottom
                }
            }
        };
        self.current_index = Some(prev);
        Some(&self.candidates[prev])
    }

    pub fn ensure_scroll_visible(&mut self, max_visible_rows: usize) {
        let cols = self.num_cols.max(1);
        let total_rows = self.candidates.len().div_ceil(cols);
        if total_rows <= max_visible_rows || max_visible_rows == 0 {
            self.scroll_row = 0;
            return;
        }
        let sel_row = self.current_index.unwrap_or(0) / cols;
        if sel_row < self.scroll_row {
            self.scroll_row = sel_row;
        } else if sel_row >= self.scroll_row + max_visible_rows {
            self.scroll_row = sel_row + 1 - max_visible_rows;
        }
        let max_scroll = total_rows.saturating_sub(max_visible_rows);
        if self.scroll_row > max_scroll {
            self.scroll_row = max_scroll;
        }
    }
}

fn replace_line_range(
    line: &mut Vec<char>,
    start: usize,
    len: usize,
    replacement: &str,
    suffix: Option<char>,
) -> usize {
    for _ in 0..len {
        if start < line.len() {
            line.remove(start);
        }
    }
    let mut insert_pos = start;
    for c in replacement.chars() {
        line.insert(insert_pos, c);
        insert_pos += 1;
    }
    if let Some(s) = suffix {
        line.insert(insert_pos, s);
        insert_pos += 1;
    }
    insert_pos
}

/// Подставляет временный предпросмотр кандидата в строку (без завершающего пробела)
pub fn apply_preview_to_line(
    line: &mut Vec<char>,
    token_start: usize,
    token_len: usize,
    candidate: &CompletionItem,
) -> usize {
    let suffix = if candidate.kind == CompletionKind::Directory && !candidate.text.ends_with('/') {
        Some('/')
    } else {
        None
    };
    replace_line_range(line, token_start, token_len, &candidate.text, suffix)
}

/// Восстанавливает исходный токен пользователя при отмене выбора (Esc)
pub fn restore_original_token(
    line: &mut Vec<char>,
    token_start: usize,
    token_len: usize,
    original_token: &str,
) -> usize {
    replace_line_range(line, token_start, token_len, original_token, None)
}

/// Заменяет токен в строке на выбранный вариант автодополнения и возвращает новую позицию курсора
pub fn apply_completion_to_line(
    line: &mut Vec<char>,
    token_start: usize,
    token_len: usize,
    candidate: &CompletionItem,
) -> usize {
    let suffix = if candidate.kind == CompletionKind::Directory {
        if !candidate.text.ends_with('/') {
            Some('/')
        } else {
            None
        }
    } else if !candidate.is_no_space() {
        Some(' ')
    } else {
        None
    };
    replace_line_range(line, token_start, token_len, &candidate.text, suffix)
}

/// Собирает кандидатов автодополнения переменных окружения
pub fn collect_variable_completions(
    token: &str,
    env_vars: &[(String, String)],
) -> Vec<CompletionItem> {
    let var_prefix = token.strip_prefix('$').unwrap_or(token);
    let mut items = Vec::new();

    // Системные стандартные переменные Wunix
    let standard_vars = ["PATH", "HOME", "USER", "SHELL", "PWD", "OLDPWD", "HOSTNAME", "TERM"];
    for &std_var in &standard_vars {
        if std_var.starts_with(var_prefix) {
            items.push(CompletionItem {
                text: format!("${}", std_var),
                display_name: format!("${}", std_var),
                kind: CompletionKind::Variable,
            });
        }
    }

    // Переменные из текущего окружения шелла
    for (k, _) in env_vars {
        if k.starts_with(var_prefix) && !items.iter().any(|it| it.text[1..] == *k) {
            items.push(CompletionItem {
                text: format!("${}", k),
                display_name: format!("${}", k),
                kind: CompletionKind::Variable,
            });
        }
    }

    items.sort_by(|a, b| a.text.cmp(&b.text));
    items
}

/// Собирает кандидатов команд (встроенные команды и исполняемые файлы)
pub fn collect_command_completions_with_bins(
    token: &str,
    bin_entries: &[(String, CompletionKind)],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Встроенные команды шелла
    for &cmd in crate::highlight::BUILTINS {
        if cmd.starts_with(token) {
            items.push(CompletionItem {
                text: cmd.to_string(),
                display_name: cmd.to_string(),
                kind: CompletionKind::Builtin,
            });
        }
    }

    // 2. Бинарники из /bin
    for (name, kind) in bin_entries {
        if name.starts_with(token) && !items.iter().any(|it| it.text == *name) {
            items.push(CompletionItem {
                text: name.clone(),
                display_name: name.clone(),
                kind: *kind,
            });
        }
    }

    // 3. Fallback: известные команды Wunix
    if bin_entries.is_empty() {
        for &cmd in crate::highlight::COMMON_COMMANDS {
            if cmd.starts_with(token) && !items.iter().any(|it| it.text == cmd) {
                items.push(CompletionItem {
                    text: cmd.to_string(),
                    display_name: cmd.to_string(),
                    kind: CompletionKind::Command,
                });
            }
        }
    }

    items.sort_by(|a, b| a.text.cmp(&b.text));
    items
}

pub use crate::parser::expand_tilde;

/// Читает содержимое директории для автодополнения
pub fn read_dir_entries(dir_path: &str) -> Vec<(String, CompletionKind)> {
    let mut entries = Vec::new();

    #[cfg(feature = "std")]
    {
        if let Ok(read_dir) = std::fs::read_dir(dir_path) {
            for entry in read_dir.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let kind = if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            CompletionKind::Directory
                        } else if ft.is_symlink() {
                            CompletionKind::Symlink
                        } else {
                            CompletionKind::File
                        }
                    } else {
                        CompletionKind::File
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
                            4 => CompletionKind::Directory,
                            10 => CompletionKind::Symlink,
                            _ => CompletionKind::File,
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

fn collect_path_completions_filtered(
    token: &str,
    home: &str,
    dirs_only: bool,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let (dir_part, prefix, token_prefix) = split_path_for_completion(token);
    let resolved_dir = expand_tilde(dir_part, home);

    let entries = read_dir_entries(&resolved_dir);
    for (name, kind) in entries {
        if (name == "." || name == "..") && !prefix.starts_with('.') {
            continue;
        }
        if dirs_only && kind != CompletionKind::Directory {
            let full_fs_path = if resolved_dir == "/" {
                format!("/{}", name)
            } else {
                format!("{}/{}", resolved_dir.trim_end_matches('/'), name)
            };
            if kind != CompletionKind::Symlink || !crate::platform::is_dir(&full_fs_path) {
                continue;
            }
        }
        if name.starts_with(prefix) {
            let full_text = format!("{}{}", token_prefix, name);
            let effective_kind = if dirs_only {
                CompletionKind::Directory
            } else {
                kind
            };
            items.push(CompletionItem {
                text: full_text,
                display_name: name,
                kind: effective_kind,
            });
        }
    }

    items.sort_by(|a, b| a.text.cmp(&b.text));
    items
}

/// Собирает кандидатов автодополнения файловых путей
pub fn collect_path_completions(token: &str, home: &str) -> Vec<CompletionItem> {
    collect_path_completions_filtered(token, home, false)
}

/// Собирает кандидатов автодополнения только каталогов
pub fn collect_dir_completions(token: &str, home: &str) -> Vec<CompletionItem> {
    collect_path_completions_filtered(token, home, true)
}

/// Собирает список исполняемых команд из `$PATH` (и `/bin` по умолчанию)
pub fn collect_all_commands(token: &str) -> Vec<CompletionItem> {
    if token.contains('/') {
        let home = crate::platform::get_env("HOME").unwrap_or_else(|| String::from("/root"));
        return collect_path_completions(token, &home);
    }

    #[cfg(test)]
    let bin_entries: Vec<(String, CompletionKind)> = Vec::new();
    #[cfg(not(test))]
    let bin_entries = {
        let path_env = crate::platform::get_env("PATH")
            .unwrap_or_else(|| String::from("/bin:/usr/bin"));
        let mut collected = Vec::new();
        for dir in path_env.split(':') {
            let trimmed = dir.trim();
            if trimmed.is_empty() {
                continue;
            }
            for (name, kind) in read_dir_entries(trimmed) {
                if kind == CompletionKind::Directory || name == "." || name == ".." {
                    continue;
                }
                if !collected.iter().any(|(n, _): &(String, CompletionKind)| *n == name) {
                    collected.push((name, CompletionKind::Command));
                }
            }
        }
        collected
    };

    collect_command_completions_with_bins(token, &bin_entries)
}

/// Главная функция сбора кандидатов для строки ввода в контексте
pub fn collect_candidates(
    line_before: &str,
    current_token: &str,
    home: &str,
    env_vars: &[(String, String)],
) -> Vec<CompletionItem> {
    let context = detect_completion_context(line_before, current_token);
    match context {
        CompletionContext::Variable => collect_variable_completions(current_token, env_vars),
        CompletionContext::Command => collect_all_commands(current_token),
        CompletionContext::Path => collect_path_completions(current_token, home),
    }
}

/// Форматирует элемент автодополнения цветом терминала ANSI
pub fn format_candidate_for_display(item: &CompletionItem) -> String {
    match item.kind {
        CompletionKind::Directory => format!("\x1b[1;34m{}/\x1b[0m", item.display_name),
        CompletionKind::Symlink => format!("\x1b[1;36m{}@\x1b[0m", item.display_name),
        CompletionKind::Command | CompletionKind::Builtin => {
            format!("\x1b[1;32m{}\x1b[0m", item.display_name)
        }
        CompletionKind::Subcommand => format!("\x1b[1;36m{}\x1b[0m", item.display_name),
        CompletionKind::Option => format!("\x1b[33m{}\x1b[0m", item.display_name),
        CompletionKind::Variable => format!("\x1b[1;33m{}\x1b[0m", item.display_name),
        CompletionKind::Argument => format!("\x1b[1;35m{}\x1b[0m", item.display_name),
        CompletionKind::File => item.display_name.clone(),
    }
}

fn display_suffix_len(kind: CompletionKind) -> usize {
    if kind == CompletionKind::Directory || kind == CompletionKind::Symlink {
        1
    } else {
        0
    }
}

fn truncate_display_label(item: &CompletionItem, max_chars: usize) -> (String, usize) {
    let suffix = match item.kind {
        CompletionKind::Directory => "/",
        CompletionKind::Symlink => "@",
        _ => "",
    };
    let name_chars = item.display_name.chars().count();
    let total_chars = name_chars + suffix.len();
    if total_chars <= max_chars || max_chars < 4 {
        return (format!("{}{}", item.display_name, suffix), total_chars);
    }
    let keep = max_chars.saturating_sub(3 + suffix.len());
    let mut clipped: String = item.display_name.chars().take(keep).collect();
    clipped.push_str("...");
    clipped.push_str(suffix);
    let len = clipped.chars().count();
    (clipped, len)
}

/// Формирует таблицу кандидатов для меню с опциональным выделением выбранного элемента.
/// Возвращает строки меню и количество колонок в сетке.
pub fn format_candidates_menu(
    items: &[CompletionItem],
    term_width: usize,
    selected: Option<usize>,
) -> (Vec<String>, usize) {
    if items.is_empty() {
        return (Vec::new(), 1);
    }

    let safe_width = term_width.max(16);
    let max_cell_cap = safe_width.saturating_sub(2).max(12);
    let max_len = items
        .iter()
        .map(|it| it.display_name.chars().count() + display_suffix_len(it.kind))
        .max()
        .unwrap_or(8)
        .min(max_cell_cap);
    let col_width = (max_len + 3).max(10).min(safe_width);
    let num_cols = (safe_width / col_width).max(1);

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut col_count = 0;

    for (idx, item) in items.iter().enumerate() {
        let is_selected = selected == Some(idx);
        let max_label = col_width.saturating_sub(2).max(6);
        let (label, raw_len) = truncate_display_label(item, max_label);

        let formatted = if is_selected {
            format!("\x1b[7m{}\x1b[0m", label)
        } else if raw_len == item.display_name.chars().count() + display_suffix_len(item.kind) {
            format_candidate_for_display(item)
        } else {
            let clipped_item = CompletionItem {
                text: item.text.clone(),
                display_name: label
                    .trim_end_matches('/')
                    .trim_end_matches('@')
                    .to_string(),
                kind: item.kind,
            };
            format_candidate_for_display(&clipped_item)
        };

        let pad = col_width.saturating_sub(raw_len);
        current_line.push_str(&formatted);
        if col_count + 1 < num_cols && idx + 1 < items.len() {
            for _ in 0..pad {
                current_line.push(' ');
            }
        }
        col_count += 1;

        if col_count >= num_cols {
            lines.push(current_line);
            current_line = String::new();
            col_count = 0;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    (lines, num_cols)
}

/// Формирует таблицу строк для вывода кандидатов в несколько колонок
pub fn format_candidates_grid(items: &[CompletionItem], term_width: usize) -> Vec<String> {
    format_candidates_menu(items, term_width, None).0
}

/// Вычисляет базовый каталог и префикс для сканирования ФС
pub fn split_path_for_completion<'a>(token: &'a str) -> (&'a str, &'a str, &'a str) {
    if let Some(last_slash) = token.rfind('/') {
        let dir_part = if last_slash == 0 { "/" } else { &token[..last_slash] };
        let prefix = &token[last_slash + 1..];
        let token_prefix = &token[..=last_slash];
        (dir_part, prefix, token_prefix)
    } else {
        (".", token, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_detect_context_first_token() {
        assert_eq!(detect_completion_context("c", "c"), CompletionContext::Command);
        assert_eq!(detect_completion_context("  cat", "cat"), CompletionContext::Command);
    }

    #[test]
    fn test_detect_context_after_pipe_and_chains() {
        assert_eq!(
            detect_completion_context("ls | c", "c"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("cat /etc/passwd | gr", "gr"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("echo 1 && ca", "ca"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("echo 1 || ca", "ca"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("pwd ; ca", "ca"),
            CompletionContext::Command
        );
    }

    #[test]
    fn test_detect_context_after_command_wrappers() {
        assert_eq!(
            detect_completion_context("sudo re", "re"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("time ls", "ls"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("which ec", "ec"),
            CompletionContext::Command
        );
        assert_eq!(
            detect_completion_context("man ca", "ca"),
            CompletionContext::Command
        );
    }

    #[test]
    fn test_detect_context_variable() {
        assert_eq!(
            detect_completion_context("echo $P", "$P"),
            CompletionContext::Variable
        );
        assert_eq!(
            detect_completion_context("export FOO=$H", "$H"),
            CompletionContext::Variable
        );
    }

    #[test]
    fn test_detect_context_path_arguments() {
        assert_eq!(
            detect_completion_context("cat /etc/p", "/etc/p"),
            CompletionContext::Path
        );
        assert_eq!(
            detect_completion_context("ls -l /v", "/v"),
            CompletionContext::Path
        );
    }

    #[test]
    fn test_find_common_prefix() {
        let v1 = vec!["clear".to_string(), "clean".to_string(), "clever".to_string()];
        assert_eq!(find_common_prefix(&v1), "cle");

        let v2 = vec!["shutdown".to_string()];
        assert_eq!(find_common_prefix(&v2), "shutdown");

        let v3: Vec<String> = vec![];
        assert_eq!(find_common_prefix(&v3), "");
    }

    #[test]
    fn test_completion_state_cycling() {
        let mut state = CompletionState::new();
        state.candidates = vec![
            CompletionItem {
                text: "cat".to_string(),
                display_name: "cat".to_string(),
                kind: CompletionKind::Command,
            },
            CompletionItem {
                text: "cd".to_string(),
                display_name: "cd".to_string(),
                kind: CompletionKind::Builtin,
            },
            CompletionItem {
                text: "clear".to_string(),
                display_name: "clear".to_string(),
                kind: CompletionKind::Builtin,
            },
        ];

        // 1-й cycle_next дает первый элемент
        assert_eq!(state.cycle_next().unwrap().text, "cat");
        // 2-й cycle_next дает второй
        assert_eq!(state.cycle_next().unwrap().text, "cd");
        // 3-й
        assert_eq!(state.cycle_next().unwrap().text, "clear");
        // Замыкание кольца: снова первый
        assert_eq!(state.cycle_next().unwrap().text, "cat");

        // Обратный ход (Shift+Tab)
        assert_eq!(state.cycle_prev().unwrap().text, "clear");
    }

    #[test]
    fn test_apply_completion_to_line_file_and_dir() {
        let mut line: Vec<char> = "cat fil".chars().collect();
        let item = CompletionItem {
            text: "file.txt".to_string(),
            display_name: "file.txt".to_string(),
            kind: CompletionKind::File,
        };
        let new_cursor = apply_completion_to_line(&mut line, 4, 3, &item);
        let res: String = line.iter().collect();
        assert_eq!(res, "cat file.txt ");
        assert_eq!(new_cursor, 13);

        // Каталог должен завершаться '/'
        let mut line2: Vec<char> = "cd doc".chars().collect();
        let dir_item = CompletionItem {
            text: "docs".to_string(),
            display_name: "docs".to_string(),
            kind: CompletionKind::Directory,
        };
        let new_cursor2 = apply_completion_to_line(&mut line2, 3, 3, &dir_item);
        let res2: String = line2.iter().collect();
        assert_eq!(res2, "cd docs/");
        assert_eq!(new_cursor2, 8);
    }

    #[test]
    fn test_variable_completions() {
        let envs = vec![
            ("EDITOR".to_string(), "nano".to_string()),
            ("EXTRA_VAR".to_string(), "1".to_string()),
        ];
        let items = collect_variable_completions("$E", &envs);
        let names: Vec<&str> = items.iter().map(|it| it.text.as_str()).collect();
        assert!(names.contains(&"$EDITOR"));
        assert!(names.contains(&"$EXTRA_VAR"));
    }

    #[test]
    fn test_expand_tilde() {
        assert_eq!(expand_tilde("~", "/root"), "/root");
        assert_eq!(expand_tilde("~/docs", "/root"), "/root/docs");
        assert_eq!(expand_tilde("~/docs", "/"), "/docs");
        assert_eq!(expand_tilde("/etc", "/root"), "/etc");
    }

    #[test]
    fn test_split_path_for_completion() {
        assert_eq!(split_path_for_completion("file"), (".", "file", ""));
        assert_eq!(split_path_for_completion("/etc/p"), ("/etc", "p", "/etc/"));
        assert_eq!(split_path_for_completion("/b"), ("/", "b", "/"));
        assert_eq!(split_path_for_completion("dir/sub/"), ("dir/sub", "", "dir/sub/"));
    }

    #[test]
    fn test_format_candidate_colors() {
        let dir = CompletionItem {
            text: "/etc".to_string(),
            display_name: "etc".to_string(),
            kind: CompletionKind::Directory,
        };
        assert_eq!(format_candidate_for_display(&dir), "\x1b[1;34metc/\x1b[0m");

        let cmd = CompletionItem {
            text: "cat".to_string(),
            display_name: "cat".to_string(),
            kind: CompletionKind::Command,
        };
        assert_eq!(format_candidate_for_display(&cmd), "\x1b[1;32mcat\x1b[0m");

        let var = CompletionItem {
            text: "$PATH".to_string(),
            display_name: "$PATH".to_string(),
            kind: CompletionKind::Variable,
        };
        assert_eq!(format_candidate_for_display(&var), "\x1b[1;33m$PATH\x1b[0m");
    }

    #[test]
    fn test_format_candidates_grid_columns() {
        let items = vec![
            CompletionItem {
                text: "alpha".to_string(),
                display_name: "alpha".to_string(),
                kind: CompletionKind::File,
            },
            CompletionItem {
                text: "beta".to_string(),
                display_name: "beta".to_string(),
                kind: CompletionKind::File,
            },
        ];
        let lines = format_candidates_grid(&items, 80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("alpha"));
        assert!(lines[0].contains("beta"));
    }

    #[test]
    fn test_collect_candidates_command_context() {
        let envs = vec![];
        let candidates = collect_candidates("sudo c", "c", "/root", &envs);
        let names: Vec<&str> = candidates.iter().map(|c| c.text.as_str()).collect();
        // Должны присутствовать builtin-команды cd, clear, cls
        assert!(names.contains(&"cd"));
        assert!(names.contains(&"clear"));
    }

    #[test]
    fn test_parse_completion_input_utf8_and_segments() {
        let chars: Vec<char> = "echo привет мир".chars().collect();
        let parsed = parse_completion_input(&chars);
        assert_eq!(parsed.prev_words, vec!["echo", "привет"]);
        assert_eq!(parsed.current_token, "мир");
        assert_eq!(parsed.token_start, 12);

        let chars2: Vec<char> = "ls /etc | wpm ins".chars().collect();
        let parsed2 = parse_completion_input(&chars2);
        assert_eq!(parsed2.prev_words, vec!["wpm"]);
        assert_eq!(parsed2.current_token, "ins");
        assert_eq!(parsed2.token_start, 14);

        let chars3: Vec<char> = "system ".chars().collect();
        let parsed3 = parse_completion_input(&chars3);
        assert_eq!(parsed3.prev_words, vec!["system"]);
        assert_eq!(parsed3.current_token, "");
        assert_eq!(parsed3.token_start, 7);
    }

    #[test]
    fn test_completion_state_2d_navigation() {
        let mut state = CompletionState::new();
        state.candidates = (0..5)
            .map(|i| CompletionItem {
                text: format!("item{}", i),
                display_name: format!("item{}", i),
                kind: CompletionKind::File,
            })
            .collect();
        state.num_cols = 3;

        // Grid:
        // 0 1 2
        // 3 4
        assert_eq!(state.move_down().unwrap().text, "item0");
        assert_eq!(state.move_down().unwrap().text, "item3");
        assert_eq!(state.move_down().unwrap().text, "item0"); // wrap column 0
        assert_eq!(state.move_right().unwrap().text, "item1");
        assert_eq!(state.move_down().unwrap().text, "item4");
        assert_eq!(state.move_up().unwrap().text, "item1");
        assert_eq!(state.move_up().unwrap().text, "item4"); // wrap column 1 to bottom
        assert_eq!(state.move_left().unwrap().text, "item3");
    }

    #[test]
    fn test_apply_preview_and_restore_original() {
        let mut line: Vec<char> = "wpm in --yes".chars().collect();
        let item = CompletionItem {
            text: "install".to_string(),
            display_name: "install".to_string(),
            kind: CompletionKind::Subcommand,
        };
        let cursor = apply_preview_to_line(&mut line, 4, 2, &item);
        assert_eq!(line.iter().collect::<String>(), "wpm install --yes");
        assert_eq!(cursor, 11);

        let restored = restore_original_token(&mut line, 4, cursor - 4, "in");
        assert_eq!(line.iter().collect::<String>(), "wpm in --yes");
        assert_eq!(restored, 6);
    }

    #[test]
    fn test_format_candidates_menu_highlights_selected() {
        let items = vec![
            CompletionItem {
                text: "alpha".to_string(),
                display_name: "alpha".to_string(),
                kind: CompletionKind::File,
            },
            CompletionItem {
                text: "beta".to_string(),
                display_name: "beta".to_string(),
                kind: CompletionKind::File,
            },
        ];
        let (lines, cols) = format_candidates_menu(&items, 80, Some(1));
        assert_eq!(cols >= 2, true);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("\x1b[7m"));
        assert!(lines[0].contains("beta"));
    }
}
