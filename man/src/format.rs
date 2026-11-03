use alloc::string::String;
use alloc::vec::Vec;

use crate::roff::{RoffMeta, RoffNode, StyleSpan};

pub struct TerminalFormatter {
    pub width: usize,
    pub ascii_only: bool,
    pub is_ru: bool,
}

impl TerminalFormatter {
    pub fn new(width: usize, ascii_only: bool, is_ru: bool) -> Self {
        Self {
            width: width.max(40),
            ascii_only,
            is_ru,
        }
    }

    pub fn format_page(&self, nodes: &[RoffNode]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_indent = 7usize;
        let mut in_no_fill = false;
        let mut pending_meta: Option<RoffMeta> = None;
        let mut pending_tag: Option<(Vec<StyleSpan>, usize)> = None;

        for node in nodes {
            match node {
                RoffNode::Title(meta) => {
                    pending_meta = Some(meta.clone());
                    lines.push(self.format_header_line(meta));
                    lines.push(String::new());
                }
                RoffNode::Section(name) => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
                    }
                    self.ensure_blank_line(&mut lines);
                    let translated = if self.is_ru {
                        Self::translate_section_heading(name)
                    } else {
                        name.as_str()
                    };
                    lines.push(self.render_bold(translated));
                    current_indent = 7;
                }
                RoffNode::SubSection(name) => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
                    }
                    self.ensure_blank_line(&mut lines);
                    let mut line = String::from("   ");
                    let translated = if self.is_ru {
                        Self::translate_section_heading(name)
                    } else {
                        name.as_str()
                    };
                    line.push_str(&self.render_bold(translated));
                    lines.push(line);
                    current_indent = 7;
                }
                RoffNode::ParagraphBreak => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
                    }
                    self.ensure_blank_line(&mut lines);
                }
                RoffNode::TaggedParagraph { tag, indent } => {
                    if let Some((prev_tag, prev_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &prev_tag, prev_indent, current_indent);
                    }
                    self.ensure_blank_line(&mut lines);
                    pending_tag = Some((tag.clone(), *indent));
                }
                RoffNode::IndentedParagraph { marker, indent } => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
                    }
                    self.ensure_blank_line(&mut lines);
                    if let Some(m) = marker {
                        let tag_str = self.render_spans(m);
                        let mut line = String::new();
                        for _ in 0..current_indent {
                            line.push(' ');
                        }
                        line.push_str(&tag_str);
                        lines.push(line);
                    }
                    current_indent = current_indent.saturating_add(*indent);
                }
                RoffNode::RelativeMargin(shift) => {
                    if *shift >= 0 {
                        current_indent = current_indent.saturating_add(*shift as usize);
                    } else {
                        current_indent = current_indent.saturating_sub((-shift) as usize).max(7);
                    }
                }
                RoffNode::LineBreak => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
                    }
                    lines.push(String::new());
                }
                RoffNode::VerticalSpace(count) => {
                    for _ in 0..*count {
                        lines.push(String::new());
                    }
                }
                RoffNode::NoFill(val) => {
                    in_no_fill = *val;
                }
                RoffNode::Text(spans) => {
                    if let Some((tag, tag_indent)) = pending_tag.take() {
                        self.render_tagged_body(&mut lines, &tag, tag_indent, current_indent, spans);
                    } else if in_no_fill {
                        let mut line = String::new();
                        for _ in 0..current_indent {
                            line.push(' ');
                        }
                        line.push_str(&self.render_spans(spans));
                        lines.push(line);
                    } else {
                        let wrapped = self.wrap_spans(spans, current_indent);
                        lines.extend(wrapped);
                    }
                }
            }
        }

        if let Some((tag, tag_indent)) = pending_tag.take() {
            self.flush_tag(&mut lines, &tag, tag_indent, current_indent);
        }

        if let Some(meta) = pending_meta {
            self.ensure_blank_line(&mut lines);
            lines.push(self.format_footer_line(&meta));
        }

        lines
    }

    fn ensure_blank_line(&self, lines: &mut Vec<String>) {
        if let Some(last) = lines.last() {
            if !last.trim().is_empty() {
                lines.push(String::new());
            }
        }
    }

    fn flush_tag(&self, lines: &mut Vec<String>, tag: &[StyleSpan], _tag_indent: usize, body_indent: usize) {
        let tag_str = self.render_spans(tag);
        let mut line = String::new();
        for _ in 0..body_indent {
            line.push(' ');
        }
        line.push_str(&tag_str);
        lines.push(line);
    }

    fn render_tagged_body(
        &self,
        lines: &mut Vec<String>,
        tag: &[StyleSpan],
        tag_indent: usize,
        body_indent: usize,
        spans: &[StyleSpan],
    ) {
        let tag_str = self.render_spans(tag);
        let tag_vis_len = Self::visible_width(&tag_str);
        let total_indent = body_indent.saturating_add(tag_indent);

        if body_indent + tag_vis_len + 1 < total_indent {
            // Тег короткий - начинаем тело на той же строке
            let mut line = String::new();
            for _ in 0..body_indent {
                line.push(' ');
            }
            line.push_str(&tag_str);
            while line.chars().count() < total_indent {
                line.push(' ');
            }

            let wrapped = self.wrap_spans(spans, total_indent);
            if let Some(first) = wrapped.first() {
                let trimmed = first.trim_start();
                line.push_str(trimmed);
                lines.push(line);
                lines.extend(wrapped.into_iter().skip(1));
            } else {
                lines.push(line);
            }
        } else {
            // Тег длинный - выводим его на отдельной строке, а тело переносим
            let mut line = String::new();
            for _ in 0..body_indent {
                line.push(' ');
            }
            line.push_str(&tag_str);
            lines.push(line);

            let wrapped = self.wrap_spans(spans, total_indent);
            lines.extend(wrapped);
        }
    }

    pub fn format_header_line(&self, meta: &RoffMeta) -> String {
        let left = alloc::format!("{}({})", meta.title.to_uppercase(), meta.section);
        let right = left.clone();
        let manual = if self.is_ru {
            Self::translate_manual_section(&meta.manual, &meta.section)
        } else {
            meta.manual.as_str()
        };

        self.align_three_columns(&left, manual, &right, true)
    }

    pub fn translate_manual_section<'a>(manual: &'a str, section: &str) -> &'a str {
        match manual {
            "User Commands" | "General Commands Manual" => "Команды пользователя",
            "System Calls" | "System Calls Manual" => "Системные вызовы",
            "Library Functions" | "Library Functions Manual" => "Библиотечные функции",
            "Special Files" => "Специальные файлы",
            "File Formats" | "File Formats Manual" => "Форматы файлов",
            "Games" | "Games Manual" => "Игры",
            "Miscellaneous" | "Miscellaneous Information Manual" => "Разное",
            "System Administration Commands" | "System Manager's Manual" => "Команды системного администрирования",
            _ => match section {
                "1" => "Команды пользователя",
                "2" => "Системные вызовы",
                "3" => "Библиотечные функции",
                "4" => "Специальные файлы",
                "5" => "Форматы файлов",
                "6" => "Игры",
                "7" => "Разное",
                "8" => "Команды системного администрирования",
                _ => manual,
            },
        }
    }

    pub fn translate_section_heading(name: &str) -> &str {
        match name.trim() {
            "NAME" => "ИМЯ",
            "SYNOPSIS" => "СИНТАКСИС",
            "DESCRIPTION" => "ОПИСАНИЕ",
            "OPTIONS" => "ОПЦИИ",
            "EXAMPLES" => "ПРИМЕРЫ",
            "EXIT STATUS" => "КОДЫ ВОЗВРАТА",
            "RETURN VALUE" => "ВОЗВРАЩАЕМОЕ ЗНАЧЕНИЕ",
            "FILES" => "ФАЙЛЫ",
            "ENVIRONMENT" => "ОКРУЖЕНИЕ",
            "SEE ALSO" => "СМ. ТАКЖЕ",
            "COMMANDS" => "КОМАНДЫ",
            "BUILT-IN COMMANDS" => "ВСТРОЕННЫЕ КОМАНДЫ",
            "SECTIONS" => "РАЗДЕЛЫ",
            "BUGS" => "ОШИБКИ",
            "AUTHOR" => "АВТОР",
            "COPYRIGHT" => "АВТОРСКИЕ ПРАВА",
            "REPORTING BUGS" => "СООБЩЕНИЕ ОБ ОШИБКАХ",
            other => other,
        }
    }

    pub fn format_footer_line(&self, meta: &RoffMeta) -> String {
        let left = &meta.source;
        let center = &meta.date;
        let right = alloc::format!("{}({})", meta.title.to_uppercase(), meta.section);

        self.align_three_columns(left, center, &right, true)
    }

    fn align_three_columns(&self, left: &str, center: &str, right: &str, bold: bool) -> String {
        let left_len = Self::visible_width(left);
        let center_len = Self::visible_width(center);
        let right_len = Self::visible_width(right);

        let left_part = if bold { self.render_bold(left) } else { String::from(left) };
        let center_part = if bold { self.render_bold(center) } else { String::from(center) };
        let right_part = if bold { self.render_bold(right) } else { String::from(right) };

        if left_len + center_len + right_len + 4 > self.width {
            return alloc::format!("{}  {}  {}", left_part, center_part, right_part);
        }

        let total_spaces = self.width.saturating_sub(left_len + right_len);
        let left_padding = total_spaces.saturating_sub(center_len) / 2;
        let right_padding = total_spaces.saturating_sub(center_len + left_padding);

        let mut res = String::with_capacity(self.width + 32);
        res.push_str(&left_part);
        for _ in 0..left_padding {
            res.push(' ');
        }
        res.push_str(&center_part);
        for _ in 0..right_padding {
            res.push(' ');
        }
        res.push_str(&right_part);
        res
    }

    pub fn wrap_spans(&self, spans: &[StyleSpan], indent: usize) -> Vec<String> {
        let mut words: Vec<(String, usize)> = Vec::new(); // (rendered_word, visible_length)

        for span in spans {
            let parts: Vec<&str> = span.text.split(' ').collect();
            for (idx, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                let styled = self.render_span_word(part, span.bold, span.italic);
                let vis_len = Self::visible_width(part);
                words.push((styled, vis_len));
                if idx + 1 < parts.len() {
                    // Разделитель между словами
                }
            }
        }

        let mut result = Vec::new();
        let max_content_width = self.width.saturating_sub(indent).max(20);

        let mut cur_line = String::new();
        let mut cur_vis_len = 0usize;

        for (w_str, w_len) in words {
            if cur_vis_len == 0 {
                // Первое слово в строке
                cur_line.push_str(&w_str);
                cur_vis_len = w_len;
            } else if cur_vis_len + 1 + w_len <= max_content_width {
                // Помещается в текущую строку
                cur_line.push(' ');
                cur_line.push_str(&w_str);
                cur_vis_len += 1 + w_len;
            } else {
                // Переносим на следующую строку
                let mut full_line = String::new();
                for _ in 0..indent {
                    full_line.push(' ');
                }
                full_line.push_str(&cur_line);
                result.push(full_line);

                cur_line.clear();
                cur_line.push_str(&w_str);
                cur_vis_len = w_len;
            }
        }

        if !cur_line.is_empty() {
            let mut full_line = String::new();
            for _ in 0..indent {
                full_line.push(' ');
            }
            full_line.push_str(&cur_line);
            result.push(full_line);
        }

        result
    }

    pub fn render_spans(&self, spans: &[StyleSpan]) -> String {
        let mut out = String::new();
        for span in spans {
            out.push_str(&self.render_span_word(&span.text, span.bold, span.italic));
        }
        out
    }

    fn render_span_word(&self, text: &str, bold: bool, italic: bool) -> String {
        if self.ascii_only || (!bold && !italic) {
            String::from(text)
        } else if bold && italic {
            alloc::format!("\x1b[1;4m{}\x1b[0m", text)
        } else if bold {
            alloc::format!("\x1b[1m{}\x1b[0m", text)
        } else {
            alloc::format!("\x1b[4m{}\x1b[0m", text)
        }
    }

    fn render_bold(&self, text: &str) -> String {
        if self.ascii_only {
            String::from(text)
        } else {
            alloc::format!("\x1b[1m{}\x1b[0m", text)
        }
    }

    pub fn visible_width(s: &str) -> usize {
        let mut len = 0;
        let mut in_escape = false;
        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if c.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else {
                len += 1;
            }
        }
        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_visible_width() {
        let plain = "hello world";
        assert_eq!(TerminalFormatter::visible_width(plain), 11);

        let colored = "\x1b[1mhello\x1b[0m \x1b[4mworld\x1b[0m";
        assert_eq!(TerminalFormatter::visible_width(colored), 11);
    }

    #[test]
    fn test_header_line_alignment() {
        let formatter = TerminalFormatter::new(80, true, false);
        let meta = RoffMeta {
            title: String::from("CAT"),
            section: String::from("1"),
            date: String::from("2026-10-28"),
            source: String::from("Wunix"),
            manual: String::from("User Commands"),
        };
        let header = formatter.format_header_line(&meta);
        assert_eq!(header.len(), 80);
        assert!(header.starts_with("CAT(1)"));
        assert!(header.ends_with("CAT(1)"));
        assert!(header.contains("User Commands"));
    }

    #[test]
    fn test_header_and_section_russian_translation() {
        let formatter = TerminalFormatter::new(80, true, true);
        let meta = RoffMeta {
            title: String::from("CAT"),
            section: String::from("1"),
            date: String::from("2026-10-28"),
            source: String::from("Wunix"),
            manual: String::from("User Commands"),
        };
        let header = formatter.format_header_line(&meta);
        assert!(header.contains("Команды пользователя"));
        assert_eq!(TerminalFormatter::translate_section_heading("NAME"), "ИМЯ");
        assert_eq!(TerminalFormatter::translate_section_heading("OPTIONS"), "ОПЦИИ");
    }

    #[test]
    fn test_word_wrapping() {
        let formatter = TerminalFormatter::new(40, true, false);
        let spans = vec![StyleSpan::plain(
            "This is a long line that definitely needs to be wrapped onto multiple lines properly.",
        )];
        let lines = formatter.wrap_spans(&spans, 7);
        assert!(lines.len() >= 3);
        for line in &lines {
            assert!(line.starts_with("       "));
            assert!(line.len() <= 40);
        }
    }
}
