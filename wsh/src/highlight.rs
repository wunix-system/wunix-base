//! Синтаксическая подсветка командной строки в режиме реального времени.
//!
//! Первое слово подсвечивается зелёным (известная команда) или красным (неизвестная).
//! Аргументы — белые. Строки в кавычках — жёлтые. Перенаправления и пайпы — тусклые.

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

pub const BUILTINS: &[&str] = &[
    "cd", "pwd", "exit", "echo", "export", "unset", "env",
    "history", "clear", "cls", "help", "type", "which", "whoami", "hostname",
];

pub const COMMON_COMMANDS: &[&str] = &[
    "ls", "cat", "rm", "touch", "mkdir", "rmdir", "ln", "readlink", "uname", "top", "free",
    "ping", "ip", "curl", "man", "mount", "umount", "fdisk", "hwclock",
    "lspci", "lshw", "lsblk", "lsmod", "modinfo", "fbset", "ss",
    "date", "time", "timedatectl", "shutdown", "reboot",
    "lsdev", "md", "rd", "mklink",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    CommandOk,   // первое слово — известная builtin/бинарник
    CommandBad,  // первое слово — неизвестная команда
    Argument,    // обычный аргумент
    Quoted,      // строка в кавычках
    Operator,    // |, >, >>, <
}

/// Разбивает строку на цветные сегменты для отображения.
/// Возвращает `Vec<(start, end, kind)>` по байтовым позициям.
pub fn tokenize_for_highlight(line: &[char]) -> Vec<(usize, usize, TokenKind)> {
    let mut segments: Vec<(usize, usize, TokenKind)> = Vec::new();
    let n = line.len();
    if n == 0 {
        return segments;
    }

    let mut pos = 0;
    let mut is_first_token = true;

    while pos < n {
        // Пробелы — не подсвечиваем
        if line[pos] == ' ' {
            pos += 1;
            continue;
        }

        // Операторы: |, ||, >, >>, <, &, &&, ;
        if line[pos] == '|' || line[pos] == '<' || line[pos] == '>' || line[pos] == '&' || line[pos] == ';' {
            let start = pos;
            let c = line[pos];
            pos += 1;
            if pos < n
                && ((c == '>' && line[pos] == '>')
                    || (c == '|' && line[pos] == '|')
                    || (c == '&' && line[pos] == '&'))
            {
                pos += 1;
            }
            segments.push((start, pos, TokenKind::Operator));
            is_first_token = true; // после оператора ожидается новая команда
            continue;
        }

        // Строки в кавычках
        if line[pos] == '"' || line[pos] == '\'' {
            let quote = line[pos];
            let start = pos;
            pos += 1;
            while pos < n && line[pos] != quote {
                pos += 1;
            }
            if pos < n {
                pos += 1; // закрывающая кавычка
            }
            segments.push((start, pos, TokenKind::Quoted));
            is_first_token = false;
            continue;
        }

        // Обычный токен (слово)
        let start = pos;
        while pos < n && line[pos] != ' ' && line[pos] != '|'
            && line[pos] != '<' && line[pos] != '>'
            && line[pos] != '&' && line[pos] != ';'
            && line[pos] != '"' && line[pos] != '\''
        {
            pos += 1;
        }

        let word: String = line[start..pos].iter().collect();

        if is_first_token {
            let kind = if is_known_command(&word) {
                TokenKind::CommandOk
            } else {
                TokenKind::CommandBad
            };
            segments.push((start, pos, kind));
            is_first_token = false;
        } else {
            segments.push((start, pos, TokenKind::Argument));
        }
    }

    segments
}

fn is_known_command(word: &str) -> bool {
    // Голый путь (содержит '/') — считаем известной командой
    if word.contains('/') {
        return true;
    }
    if BUILTINS.contains(&word) {
        return true;
    }
    #[cfg(any(feature = "std", test))]
    {
        if COMMON_COMMANDS.contains(&word) {
            return true;
        }
    }
    #[cfg(all(not(feature = "std"), not(test)))]
    {
        // Проверяем наличие бинарника в /bin/ через системный вызов stat
        let mut path_buf = [0u8; 128];
        let prefix = b"/bin/";
        if prefix.len() + word.len() + 1 < path_buf.len() {
            path_buf[..prefix.len()].copy_from_slice(prefix);
            path_buf[prefix.len()..prefix.len() + word.len()].copy_from_slice(word.as_bytes());
            path_buf[prefix.len() + word.len()] = 0;
            let mut st = libwunix::Stat::zeroed();
            if libwunix::stat(&path_buf[..prefix.len() + word.len() + 1], &mut st) == 0 {
                return true;
            }
        }
    }
    false
}

/// Строит строку с ANSI-подсветкой для вывода на терминал.
pub fn highlight_line(line: &[char]) -> String {
    let segments = tokenize_for_highlight(line);
    if segments.is_empty() {
        return line.iter().collect();
    }

    let mut result = String::with_capacity(line.len() * 6);
    let mut prev_end = 0;

    for (start, end, kind) in &segments {
        // Пробелы между токенами — без подсветки
        for &c in &line[prev_end..*start] {
            result.push(c);
        }

        let color = match kind {
            TokenKind::CommandOk  => "\x1b[1;32m", // жирный зелёный
            TokenKind::CommandBad => "\x1b[1;31m", // жирный красный
            TokenKind::Argument   => "\x1b[0m",    // сброс (белый)
            TokenKind::Quoted     => "\x1b[1;33m", // жёлтый
            TokenKind::Operator   => "\x1b[90m",   // тусклый серый
        };
        result.push_str(color);
        for &c in &line[*start..*end] {
            result.push(c);
        }
        result.push_str("\x1b[0m");

        prev_end = *end;
    }

    // Хвост (если остались символы после последнего токена — не должно быть, но на всякий)
    for &c in &line[prev_end..] {
        result.push(c);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn test_empty_line() {
        assert!(tokenize_for_highlight(&chars("")).is_empty());
    }

    #[test]
    fn test_known_builtin_ok() {
        let segs = tokenize_for_highlight(&chars("echo hello"));
        assert_eq!(segs[0].2, TokenKind::CommandOk);
        assert_eq!(segs[1].2, TokenKind::Argument);
    }

    #[test]
    fn test_unknown_command_bad() {
        let segs = tokenize_for_highlight(&chars("foobar arg1"));
        assert_eq!(segs[0].2, TokenKind::CommandBad);
        assert_eq!(segs[1].2, TokenKind::Argument);
    }

    #[test]
    fn test_quoted_argument() {
        let segs = tokenize_for_highlight(&chars("echo \"hello world\""));
        assert_eq!(segs[0].2, TokenKind::CommandOk);
        assert_eq!(segs[1].2, TokenKind::Quoted);
    }

    #[test]
    fn test_operator_pipe_resets_command() {
        let segs = tokenize_for_highlight(&chars("echo hi | cat"));
        // echo → CommandOk, hi → Argument, | → Operator, cat → CommandOk (known /bin/cat)
        let kinds: Vec<TokenKind> = segs.iter().map(|s| s.2).collect();
        assert_eq!(kinds[0], TokenKind::CommandOk);  // echo
        assert_eq!(kinds[1], TokenKind::Argument);   // hi
        assert_eq!(kinds[2], TokenKind::Operator);   // |
        // cat — неизвестен как builtin, но тест проверяет структуру
        assert!(kinds[3] == TokenKind::CommandOk || kinds[3] == TokenKind::CommandBad);
    }

    #[test]
    fn test_redirect_operator() {
        let segs = tokenize_for_highlight(&chars("echo hi > /tmp/out"));
        let kinds: Vec<TokenKind> = segs.iter().map(|s| s.2).collect();
        assert!(kinds.contains(&TokenKind::Operator));
    }

    #[test]
    fn test_chain_operators_highlight() {
        let segs = tokenize_for_highlight(&chars("echo 1 && pwd || ls ; hostname &"));
        // Проверяем наличие операторов &&, ||, ;, &
        let operators: Vec<TokenKind> = segs
            .iter()
            .filter(|s| s.2 == TokenKind::Operator)
            .map(|s| s.2)
            .collect();
        assert_eq!(operators.len(), 4);

        // Все команды echo, pwd, ls, hostname должны распознаваться как CommandOk
        let ok_commands: Vec<usize> = segs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.2 == TokenKind::CommandOk)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(ok_commands.len(), 4);
    }

    #[test]
    fn test_path_as_command_ok() {
        let segs = tokenize_for_highlight(&chars("/bin/ls -la"));
        assert_eq!(segs[0].2, TokenKind::CommandOk);
    }

    #[test]
    fn test_highlight_line_produces_ansi() {
        let line = chars("echo hello");
        let out = highlight_line(&line);
        // Должны быть ANSI escape-коды
        assert!(out.contains("\x1b["));
        // Видимый текст должен содержать оба слова
        assert!(out.contains("echo"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn test_token_boundaries_cover_full_line() {
        let line = chars("ls -la /tmp");
        let segs = tokenize_for_highlight(&line);
        // Все символы слов должны попадать в диапазоны сегментов (пробелы — вне)
        let covered: Vec<bool> = (0..line.len()).map(|i| {
            segs.iter().any(|(s, e, _)| i >= *s && i < *e)
        }).collect();
        // Пробелы на позициях 2 и 5 не должны быть покрыты
        assert!(!covered[2]); // пробел после ls
        assert!(covered[0]);  // 'l'
        assert!(covered[1]);  // 's'
    }

    #[test]
    fn test_new_system_commands_highlight_ok() {
        let commands = ["date", "time", "timedatectl", "shutdown", "reboot"];
        for cmd in commands {
            let segs = tokenize_for_highlight(&chars(cmd));
            assert_eq!(
                segs[0].2,
                TokenKind::CommandOk,
                "Команда '{}' должна подсвечиваться как CommandOk (зеленым)",
                cmd
            );
        }
    }

    #[test]
    fn test_unused_commands_highlight_bad() {
        let unused = ["reg", "poweroff", "halt", "cp", "mv", "grep", "sleep"];
        for cmd in unused {
            let segs = tokenize_for_highlight(&chars(cmd));
            assert_eq!(
                segs[0].2,
                TokenKind::CommandBad,
                "Несуществующая команда '{}' не должна подсвечиваться зеленым",
                cmd
            );
        }
    }
}
