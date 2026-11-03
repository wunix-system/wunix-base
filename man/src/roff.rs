use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

impl StyleSpan {
    pub fn new(text: String, bold: bool, italic: bool) -> Self {
        Self { text, bold, italic }
    }

    pub fn plain(text: &str) -> Self {
        Self {
            text: String::from(text),
            bold: false,
            italic: false,
        }
    }

    pub fn bold(text: &str) -> Self {
        Self {
            text: String::from(text),
            bold: true,
            italic: false,
        }
    }

    pub fn italic(text: &str) -> Self {
        Self {
            text: String::from(text),
            bold: false,
            italic: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoffMeta {
    pub title: String,
    pub section: String,
    pub date: String,
    pub source: String,
    pub manual: String,
}

impl Default for RoffMeta {
    fn default() -> Self {
        Self {
            title: String::new(),
            section: String::from("1"),
            date: String::new(),
            source: String::from("Wunix"),
            manual: String::from("User Commands"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoffNode {
    Title(RoffMeta),
    Section(String),
    SubSection(String),
    ParagraphBreak,
    TaggedParagraph {
        tag: Vec<StyleSpan>,
        indent: usize,
    },
    IndentedParagraph {
        marker: Option<Vec<StyleSpan>>,
        indent: usize,
    },
    RelativeMargin(isize),
    LineBreak,
    VerticalSpace(usize),
    NoFill(bool),
    Text(Vec<StyleSpan>),
}

pub struct RoffParser;

impl RoffParser {
    pub fn parse(input: &str) -> Vec<RoffNode> {
        let mut nodes = Vec::new();
        let mut lines = input.lines().peekable();
        let mut in_no_fill = false;

        while let Some(line) = lines.next() {
            let trimmed = line.trim_end();
            if trimmed.starts_with(".\\\"") || trimmed.starts_with("'\\\"") {
                // Комментарий roff - пропускаем
                continue;
            }

            if trimmed.starts_with('.') || (trimmed.starts_with('\'') && !in_no_fill) {
                let macro_line = &trimmed[1..].trim_start();
                if macro_line.is_empty() {
                    continue;
                }

                let (macro_name, rest) = Self::split_macro_word(macro_line);
                match macro_name {
                    "TH" => {
                        let args = Self::parse_macro_args(rest);
                        let title = args.get(0).cloned().unwrap_or_default();
                        let section = args.get(1).cloned().unwrap_or_else(|| String::from("1"));
                        let date = args.get(2).cloned().unwrap_or_default();
                        let source = args.get(3).cloned().unwrap_or_else(|| String::from("Wunix"));
                        let manual = args.get(4).cloned().unwrap_or_else(|| Self::default_manual_name(&section));
                        nodes.push(RoffNode::Title(RoffMeta {
                            title,
                            section,
                            date,
                            source,
                            manual,
                        }));
                    }
                    "SH" => {
                        let heading = Self::clean_quotes(rest.trim());
                        nodes.push(RoffNode::Section(heading));
                    }
                    "SS" => {
                        let sub = Self::clean_quotes(rest.trim());
                        nodes.push(RoffNode::SubSection(sub));
                    }
                    "PP" | "P" | "LP" => {
                        nodes.push(RoffNode::ParagraphBreak);
                    }
                    "TP" => {
                        let indent = rest.trim().parse::<usize>().unwrap_or(7);
                        // Следующая непустая и не являющаяся комментарием строка является тегом .TP
                        let mut tag_spans = Vec::new();
                        while let Some(&next_line) = lines.peek() {
                            let n_trimmed = next_line.trim_end();
                            if n_trimmed.starts_with(".\\\"") || n_trimmed.starts_with("'\\\"") {
                                lines.next();
                                continue;
                            }
                            if n_trimmed.starts_with('.') {
                                let inner_macro = &n_trimmed[1..].trim_start();
                                let (inner_name, inner_rest) = Self::split_macro_word(inner_macro);
                                if inner_name == "B" || inner_name == "I" || inner_name == "BR" || inner_name == "BI" {
                                    lines.next();
                                    tag_spans = Self::parse_font_macro(inner_name, inner_rest);
                                    break;
                                }
                            }
                            lines.next();
                            tag_spans = Self::parse_inline_spans(n_trimmed);
                            break;
                        }
                        nodes.push(RoffNode::TaggedParagraph {
                            tag: tag_spans,
                            indent,
                        });
                    }
                    "IP" => {
                        let args = Self::parse_macro_args(rest);
                        let marker = args.get(0).map(|m| Self::parse_inline_spans(m));
                        let indent = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(7);
                        nodes.push(RoffNode::IndentedParagraph { marker, indent });
                    }
                    "HP" => {
                        let indent = rest.trim().parse::<usize>().unwrap_or(7);
                        nodes.push(RoffNode::IndentedParagraph { marker: None, indent });
                    }
                    "RS" => {
                        let shift = rest.trim().parse::<isize>().unwrap_or(7);
                        nodes.push(RoffNode::RelativeMargin(shift));
                    }
                    "RE" => {
                        nodes.push(RoffNode::RelativeMargin(-7));
                    }
                    "br" => {
                        nodes.push(RoffNode::LineBreak);
                    }
                    "sp" => {
                        let count = rest.trim().parse::<usize>().unwrap_or(1);
                        nodes.push(RoffNode::VerticalSpace(count));
                    }
                    "nf" => {
                        in_no_fill = true;
                        nodes.push(RoffNode::NoFill(true));
                    }
                    "fi" => {
                        in_no_fill = false;
                        nodes.push(RoffNode::NoFill(false));
                    }
                    "B" | "I" | "SM" | "BR" | "BI" | "IB" | "IR" | "RB" | "RI" => {
                        let spans = Self::parse_font_macro(macro_name, rest);
                        nodes.push(RoffNode::Text(spans));
                    }
                    _ => {
                        // Неизвестный или неподдерживаемый макрос - выводим аргументы как текст
                        if !rest.is_empty() {
                            nodes.push(RoffNode::Text(Self::parse_inline_spans(rest)));
                        }
                    }
                }
            } else {
                // Обычная текстовая строка
                if in_no_fill {
                    let spans = Self::parse_inline_spans(line);
                    // В режиме verbatim сохраняем точный вид строки
                    nodes.push(RoffNode::Text(spans));
                    nodes.push(RoffNode::LineBreak);
                } else if trimmed.is_empty() {
                    nodes.push(RoffNode::ParagraphBreak);
                } else {
                    let spans = Self::parse_inline_spans(trimmed);
                    nodes.push(RoffNode::Text(spans));
                }
            }
        }

        nodes
    }

    fn default_manual_name(sec: &str) -> String {
        match sec {
            "1" => String::from("User Commands"),
            "2" => String::from("System Calls"),
            "3" => String::from("Library Functions"),
            "4" => String::from("Special Files"),
            "5" => String::from("File Formats"),
            "6" => String::from("Games"),
            "7" => String::from("Miscellaneous"),
            "8" => String::from("System Administration Commands"),
            _ => String::from("Manual"),
        }
    }

    fn clean_quotes(s: &str) -> String {
        let trimmed = s.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            String::from(&trimmed[1..trimmed.len() - 1])
        } else {
            String::from(trimmed)
        }
    }

    fn split_macro_word(s: &str) -> (&str, &str) {
        if let Some(idx) = s.find(|c: char| c.is_whitespace()) {
            (&s[..idx], s[idx..].trim_start())
        } else {
            (s, "")
        }
    }

    pub fn parse_macro_args(s: &str) -> Vec<String> {
        let mut args = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        let len = chars.len();

        while i < len {
            // Пропускаем пробелы
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= len {
                break;
            }

            if chars[i] == '"' {
                i += 1; // Пропускаем открывающую кавычку
                let mut arg = String::new();
                while i < len && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < len && chars[i + 1] == '"' {
                        arg.push('"');
                        i += 2;
                    } else {
                        arg.push(chars[i]);
                        i += 1;
                    }
                }
                if i < len && chars[i] == '"' {
                    i += 1; // Закрывающая кавычка
                }
                args.push(arg);
            } else {
                let mut arg = String::new();
                while i < len && !chars[i].is_whitespace() {
                    arg.push(chars[i]);
                    i += 1;
                }
                args.push(arg);
            }
        }

        args
    }

    pub fn parse_font_macro(macro_name: &str, rest: &str) -> Vec<StyleSpan> {
        let args = Self::parse_macro_args(rest);
        let mut spans = Vec::new();

        match macro_name {
            "B" => {
                let text = args.join(" ");
                spans.push(StyleSpan::bold(&text));
            }
            "I" => {
                let text = args.join(" ");
                spans.push(StyleSpan::italic(&text));
            }
            "SM" => {
                let text = args.join(" ");
                spans.push(StyleSpan::plain(&text));
            }
            "BR" => {
                // Чередуются: Bold, Roman, Bold, Roman... без пробелов между аргументами
                for (idx, arg) in args.iter().enumerate() {
                    let is_bold = idx % 2 == 0;
                    spans.push(StyleSpan::new(arg.clone(), is_bold, false));
                }
            }
            "BI" => {
                for (idx, arg) in args.iter().enumerate() {
                    let is_bold = idx % 2 == 0;
                    spans.push(StyleSpan::new(arg.clone(), is_bold, !is_bold));
                }
            }
            "IB" => {
                for (idx, arg) in args.iter().enumerate() {
                    let is_italic = idx % 2 == 0;
                    spans.push(StyleSpan::new(arg.clone(), !is_italic, is_italic));
                }
            }
            "IR" => {
                for (idx, arg) in args.iter().enumerate() {
                    let is_italic = idx % 2 == 0;
                    spans.push(StyleSpan::new(arg.clone(), false, is_italic));
                }
            }
            "RB" => {
                for (idx, arg) in args.iter().enumerate() {
                    let is_bold = idx % 2 == 1;
                    spans.push(StyleSpan::new(arg.clone(), is_bold, false));
                }
            }
            "RI" => {
                for (idx, arg) in args.iter().enumerate() {
                    let is_italic = idx % 2 == 1;
                    spans.push(StyleSpan::new(arg.clone(), false, is_italic));
                }
            }
            _ => {
                let text = args.join(" ");
                spans.push(StyleSpan::plain(&text));
            }
        }

        spans
    }

    pub fn parse_inline_spans(input: &str) -> Vec<StyleSpan> {
        let mut spans = Vec::new();
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        let mut cur_text = String::new();
        let mut cur_bold = false;
        let mut cur_italic = false;

        let mut prev_bold = false;
        let mut prev_italic = false;

        while i < len {
            if chars[i] == '\\' && i + 1 < len {
                let next = chars[i + 1];
                match next {
                    'f' => {
                        // Смена шрифта \fB, \fI, \fR, \fP, \f(BI, \f1, \f2, \f3
                        if !cur_text.is_empty() {
                            spans.push(StyleSpan::new(cur_text.clone(), cur_bold, cur_italic));
                            cur_text.clear();
                        }
                        i += 2;
                        if i < len {
                            let font_code = chars[i];
                            let old_b = cur_bold;
                            let old_i = cur_italic;
                            match font_code {
                                'B' | '3' => {
                                    cur_bold = true;
                                    cur_italic = false;
                                }
                                'I' | '2' => {
                                    cur_bold = false;
                                    cur_italic = true;
                                }
                                'R' | '1' => {
                                    cur_bold = false;
                                    cur_italic = false;
                                }
                                'P' => {
                                    cur_bold = prev_bold;
                                    cur_italic = prev_italic;
                                }
                                '(' => {
                                    // \f(BI
                                    i += 1;
                                    if i + 1 < len && chars[i] == 'B' && chars[i + 1] == 'I' {
                                        cur_bold = true;
                                        cur_italic = true;
                                        i += 1;
                                    }
                                }
                                _ => {}
                            }
                            prev_bold = old_b;
                            prev_italic = old_i;
                            i += 1;
                        }
                        continue;
                    }
                    '-' => {
                        cur_text.push('-');
                        i += 2;
                        continue;
                    }
                    'e' | '\\' => {
                        cur_text.push('\\');
                        i += 2;
                        continue;
                    }
                    '&' => {
                        // Zero-width space
                        i += 2;
                        continue;
                    }
                    ' ' => {
                        // Неразрывный пробел
                        cur_text.push(' ');
                        i += 2;
                        continue;
                    }
                    '*' => {
                        // Строковая переменная типа \*(lq или \*(rq
                        i += 2;
                        if i + 2 < len && chars[i] == '(' {
                            let name: String = chars[i + 1..i + 3].iter().collect();
                            i += 3;
                            if name == "lq" || name == "rq" {
                                cur_text.push('"');
                            }
                        }
                        continue;
                    }
                    '(' => {
                        // Спецсимвол \(em, \(en, \(bu, \(aq
                        i += 2;
                        if i + 1 < len {
                            let code: String = chars[i..i + 2].iter().collect();
                            i += 2;
                            match code.as_str() {
                                "em" => cur_text.push('—'),
                                "en" => cur_text.push('–'),
                                "bu" => cur_text.push('•'),
                                "aq" => cur_text.push('\''),
                                _ => {}
                            }
                        }
                        continue;
                    }
                    _ => {
                        cur_text.push(next);
                        i += 2;
                        continue;
                    }
                }
            }

            cur_text.push(chars[i]);
            i += 1;
        }

        if !cur_text.is_empty() {
            spans.push(StyleSpan::new(cur_text, cur_bold, cur_italic));
        }

        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_th_header() {
        let roff = ".TH CAT 1 \"2026-10-28\" \"Wunix\" \"User Commands\"\n.SH NAME\ncat";
        let nodes = RoffParser::parse(roff);
        assert_eq!(
            nodes[0],
            RoffNode::Title(RoffMeta {
                title: String::from("CAT"),
                section: String::from("1"),
                date: String::from("2026-10-28"),
                source: String::from("Wunix"),
                manual: String::from("User Commands"),
            })
        );
        assert_eq!(nodes[1], RoffNode::Section(String::from("NAME")));
    }

    #[test]
    fn test_parse_font_macros() {
        let roff = ".BR cat (1)\n.B hello world\n.I italic text";
        let nodes = RoffParser::parse(roff);

        assert_eq!(
            nodes[0],
            RoffNode::Text(alloc::vec![
                StyleSpan::bold("cat"),
                StyleSpan::plain("(1)"),
            ])
        );

        assert_eq!(
            nodes[1],
            RoffNode::Text(alloc::vec![StyleSpan::bold("hello world")])
        );

        assert_eq!(
            nodes[2],
            RoffNode::Text(alloc::vec![StyleSpan::italic("italic text")])
        );
    }

    #[test]
    fn test_parse_inline_escapes() {
        let text = r"Execute \fBcat\fR \fIfile\fP \- read";
        let spans = RoffParser::parse_inline_spans(text);
        assert_eq!(
            spans,
            alloc::vec![
                StyleSpan::plain("Execute "),
                StyleSpan::bold("cat"),
                StyleSpan::plain(" "),
                StyleSpan::italic("file"),
                StyleSpan::plain(" - read"),
            ]
        );
    }

    #[test]
    fn test_parse_tp() {
        let roff = ".TP\n\\fB-n\\fR, \\fB--number\\fR\nnumber all output lines";
        let nodes = RoffParser::parse(roff);
        match &nodes[0] {
            RoffNode::TaggedParagraph { tag, indent } => {
                assert_eq!(*indent, 7);
                assert_eq!(
                    tag,
                    &alloc::vec![
                        StyleSpan::bold("-n"),
                        StyleSpan::plain(", "),
                        StyleSpan::bold("--number"),
                    ]
                );
            }
            _ => panic!("Expected TaggedParagraph"),
        }
    }
}
