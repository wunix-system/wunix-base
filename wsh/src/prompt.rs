#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::string::String;

#[cfg(feature = "std")]
use std::string::String;

pub struct Prompt;

impl Prompt {
    /// Сокращает домашнюю директорию до '~' для удобства восприятия
    pub fn format_cwd(cwd: &str, home: &str) -> String {
        if !home.is_empty() && cwd == home {
            return String::from("~");
        }
        if !home.is_empty() && cwd.starts_with(home) {
            let rest = &cwd[home.len()..];
            if rest.starts_with('/') {
                let mut s = String::from("~");
                s.push_str(rest);
                return s;
            }
        }
        String::from(cwd)
    }

    /// Возвращает видимую (печатаемую) длину строки, исключая ANSI escape-последовательности.
    /// Нужна для точного расчёта позиции курсора при перерисовке строки ввода.
    pub fn visible_len(s: &str) -> usize {
        let mut len = 0;
        let mut in_escape = false;
        for c in s.chars() {
            if in_escape {
                if c == 'm' || c == 'J' || c == 'K' || c == 'H' || c == 'A'
                    || c == 'B' || c == 'C' || c == 'D' || c == 'h' || c == 'l'
                {
                    in_escape = false;
                }
            } else if c == '\x1b' {
                in_escape = true;
            } else {
                len += 1;
            }
        }
        len
    }

    /// Формирует строку приглашения: user@hostname ~/path >
    /// Имя устройства добавлено для идентификации машины в многопользовательских и сетевых сценариях.
    pub fn build(user: &str, host: &str, cwd: &str, home: &str, is_root: bool, last_status: i32) -> String {
        let display_cwd = Self::format_cwd(cwd, home);

        // Если путь содержит букву диска (C:\...)
        if display_cwd.starts_with("C:") || display_cwd.starts_with("c:") {
            let mut s = String::new();
            s.push_str("\x1b[1;36m");
            s.push_str(&display_cwd);
            s.push_str("\x1b[0m \x1b[1;32m>\x1b[0m ");
            return s;
        }

        let mut s = String::new();

        // user@hostname (бирюзовый) — классический Unix-формат с именем устройства
        s.push_str("\x1b[1;36m");
        s.push_str(user);
        if !host.is_empty() {
            s.push('@');
            s.push_str(host);
        }
        s.push_str("\x1b[0m ");

        // Рабочая директория (яркий синий)
        s.push_str("\x1b[1;34m");
        s.push_str(&display_cwd);
        s.push_str("\x1b[0m ");

        // Индикатор статуса и приглашение к вводу
        if is_root {
            s.push_str("\x1b[1;31m#\x1b[0m ");
        } else if last_status != 0 {
            s.push_str("\x1b[1;31m[!] >\x1b[0m ");
        } else {
            s.push_str("\x1b[1;32m>\x1b[0m ");
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cwd_home() {
        assert_eq!(Prompt::format_cwd("/home/user", "/home/user"), "~");
        assert_eq!(Prompt::format_cwd("/home/user/docs", "/home/user"), "~/docs");
        assert_eq!(Prompt::format_cwd("/etc", "/home/user"), "/etc");
    }

    #[test]
    fn test_build_includes_hostname() {
        let p = Prompt::build("root", "wunix", "/root", "/root", true, 0);
        // Видимый текст должен содержать "root@wunix" и "~"
        assert!(p.contains("root@wunix"));
        assert!(p.contains('~'));
    }

    #[test]
    fn test_build_no_host_when_empty() {
        let p = Prompt::build("user", "", "/home/user", "/home/user", false, 0);
        assert!(!p.contains('@'));
        assert!(p.contains("user"));
    }

    #[test]
    fn test_visible_len_strips_ansi() {
        // "user@wunix ~ > " — видимая часть без ANSI-кодов
        let p = Prompt::build("user", "wunix", "/home/user", "/home/user", false, 0);
        // user@wunix ~ > = 15 символов
        assert_eq!(Prompt::visible_len(&p), 15);
    }

    #[test]
    fn test_visible_len_plain() {
        assert_eq!(Prompt::visible_len("hello"), 5);
        assert_eq!(Prompt::visible_len("\x1b[1;32mok\x1b[0m"), 2);
    }
}
