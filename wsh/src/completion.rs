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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub text: String,
    pub display_name: String,
    pub kind: CompletionKind,
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

/// Состояние циклического перебора вариантов при повторных нажатиях Tab
#[derive(Default)]
pub struct CompletionState {
    pub is_active: bool,
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
        self.token_start = 0;
        self.original_token.clear();
        self.candidates.clear();
        self.current_index = None;
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
}

/// Заменяет токен в строке на выбранный вариант автодополнения и возвращает новую позицию курсора
pub fn apply_completion_to_line(
    line: &mut Vec<char>,
    token_start: usize,
    token_len: usize,
    candidate: &CompletionItem,
) -> usize {
    // Удаляем старый токен
    for _ in 0..token_len {
        if token_start < line.len() {
            line.remove(token_start);
        }
    }

    // Вставляем текст кандидата
    let mut insert_pos = token_start;
    for c in candidate.text.chars() {
        line.insert(insert_pos, c);
        insert_pos += 1;
    }

    // Добавляем завершающий символ в зависимости от типа сущности:
    // Каталог завершается слешем '/', чтобы можно было сразу продолжать ввод пути.
    // Файлы, команды и переменные завершаются пробелом ' '.
    if candidate.kind == CompletionKind::Directory {
        if !candidate.text.ends_with('/') {
            line.insert(insert_pos, '/');
            insert_pos += 1;
        }
    } else {
        line.insert(insert_pos, ' ');
        insert_pos += 1;
    }

    insert_pos
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

/// Собирает кандидатов автодополнения файловых путей
pub fn collect_path_completions(token: &str, home: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let (dir_part, prefix, token_prefix) = split_path_for_completion(token);
    let resolved_dir = expand_tilde(dir_part, home);

    let entries = read_dir_entries(&resolved_dir);
    for (name, kind) in entries {
        if (name == "." || name == "..") && !prefix.starts_with('.') {
            continue;
        }
        if name.starts_with(prefix) {
            let full_text = format!("{}{}", token_prefix, name);
            items.push(CompletionItem {
                text: full_text,
                display_name: name,
                kind,
            });
        }
    }

    items.sort_by(|a, b| a.text.cmp(&b.text));
    items
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
        CompletionContext::Command => {
            if current_token.contains('/') {
                collect_path_completions(current_token, home)
            } else {
                #[cfg(test)]
                let bin_entries: Vec<(String, CompletionKind)> = Vec::new();
                #[cfg(not(test))]
                let bin_entries = read_dir_entries("/bin");

                collect_command_completions_with_bins(current_token, &bin_entries)
            }
        }
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
        CompletionKind::Variable => format!("\x1b[1;33m{}\x1b[0m", item.display_name),
        CompletionKind::File => item.display_name.clone(),
    }
}

/// Формирует таблицу строк для вывода кандидатов в несколько колонок
pub fn format_candidates_grid(items: &[CompletionItem], term_width: usize) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }

    let max_len = items
        .iter()
        .map(|it| it.display_name.len() + 1)
        .max()
        .unwrap_or(8);
    let col_width = (max_len + 3).max(12);
    let num_cols = (term_width / col_width).max(1);

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut col_count = 0;

    for item in items {
        let formatted = format_candidate_for_display(item);
        let raw_len = item.display_name.len()
            + if item.kind == CompletionKind::Directory || item.kind == CompletionKind::Symlink {
                1
            } else {
                0
            };
        let pad = col_width.saturating_sub(raw_len);

        current_line.push_str(&formatted);
        for _ in 0..pad {
            current_line.push(' ');
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

    lines
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
}
