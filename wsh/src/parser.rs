#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, string::ToString, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirect {
    Output { fd: u64, path: String, append: bool },
    Input { fd: u64, path: String },
    DupFd { src_fd: u64, dst_fd: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordSegment {
    Literal(String),
    DoubleQuoted(String),
    Unquoted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub segments: Vec<WordSegment>,
}

impl Word {
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn push_segment(&mut self, seg: WordSegment) {
        match (&mut self.segments.last_mut(), &seg) {
            (Some(WordSegment::Literal(a)), WordSegment::Literal(b)) => a.push_str(b),
            (Some(WordSegment::DoubleQuoted(a)), WordSegment::DoubleQuoted(b)) => a.push_str(b),
            (Some(WordSegment::Unquoted(a)), WordSegment::Unquoted(b)) => a.push_str(b),
            _ => self.segments.push(seg),
        }
    }

    pub fn push_char(&mut self, ch: char, quoted: QuoteMode) {
        match quoted {
            QuoteMode::Single => match self.segments.last_mut() {
                Some(WordSegment::Literal(s)) => s.push(ch),
                _ => self.segments.push(WordSegment::Literal(String::from(ch))),
            },
            QuoteMode::Double => match self.segments.last_mut() {
                Some(WordSegment::DoubleQuoted(s)) => s.push(ch),
                _ => self.segments.push(WordSegment::DoubleQuoted(String::from(ch))),
            },
            QuoteMode::None => match self.segments.last_mut() {
                Some(WordSegment::Unquoted(s)) => s.push(ch),
                _ => self.segments.push(WordSegment::Unquoted(String::from(ch))),
            },
        }
    }

    pub fn raw_text(&self) -> String {
        let mut out = String::new();
        for seg in &self.segments {
            match seg {
                WordSegment::Literal(s) | WordSegment::DoubleQuoted(s) | WordSegment::Unquoted(s) => {
                    out.push_str(s);
                }
            }
        }
        out
    }

    pub fn is_plain_operator(&self, op: &str) -> bool {
        self.segments.len() == 1
            && matches!(&self.segments[0], WordSegment::Unquoted(s) if s == op)
    }

    pub fn as_unquoted_str(&self) -> Option<&str> {
        if self.segments.len() == 1 {
            if let WordSegment::Unquoted(s) = &self.segments[0] {
                return Some(s.as_str());
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteMode {
    None,
    Single,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirect>,
}

impl SimpleCommand {
    pub fn new(name: String) -> Self {
        Self {
            name,
            args: Vec::new(),
            redirects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOp {
    Always,      // ; или конец цепочки
    And,         // &&: запускать следующую команду только при успехе (0)
    Or,          // ||: запускать следующую команду только при ошибке (!= 0)
    Background,  // &: запускать асинхронно / в фоновом режиме
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainedPipeline {
    pub pipeline: Pipeline,
    pub op: ChainOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandChain {
    pub pipelines: Vec<ChainedPipeline>,
}

/// Раскрывает тильду '~' в домашний каталог пользователя (для '~' и '~/...')
pub fn expand_tilde(path: &str, home: &str) -> String {
    if path == "~" {
        home.to_string()
    } else if let Some(sub) = path.strip_prefix("~/") {
        let base = home.trim_end_matches('/');
        let mut out = String::from(base);
        out.push('/');
        out.push_str(sub);
        out
    } else {
        path.to_string()
    }
}

pub struct Parser;

impl Parser {
    /// Токенизирует строку в структурированные слова с сохранением информации о кавычках.
    pub fn tokenize_words(input: &str) -> Vec<Word> {
        let mut words = Vec::new();
        let mut current = Word::new();
        let mut chars = input.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        let flush_word = |words: &mut Vec<Word>, current: &mut Word| {
            if !current.is_empty() {
                words.push(core::mem::replace(current, Word::new()));
            }
        };

        while let Some(c) = chars.next() {
            if in_single_quote {
                if c == '\'' {
                    in_single_quote = false;
                } else {
                    current.push_char(c, QuoteMode::Single);
                }
                continue;
            }

            let mode = if in_double_quote { QuoteMode::Double } else { QuoteMode::None };

            match c {
                '$' if !in_double_quote && chars.peek() == Some(&'\'') => {
                    chars.next(); // consume '\''
                    let decoded = parse_ansi_c_string(&mut chars);
                    current.push_segment(WordSegment::Literal(decoded));
                }
                '$' if chars.peek() == Some(&'(') => {
                    chars.next(); // consume '('
                    current.push_char('$', mode);
                    current.push_char('(', mode);
                    collect_balanced_parens(&mut chars, &mut current, mode);
                }
                '`' => {
                    current.push_char('`', mode);
                    while let Some(nc) = chars.next() {
                        current.push_char(nc, mode);
                        if nc == '\\' {
                            if let Some(esc) = chars.next() {
                                current.push_char(esc, mode);
                            }
                        } else if nc == '`' {
                            break;
                        }
                    }
                }
                '\\' => {
                    if in_double_quote {
                        match chars.peek() {
                            Some(&'"') | Some(&'\\') | Some(&'$') | Some(&'`') => {
                                if let Some(next) = chars.next() {
                                    current.push_char(next, QuoteMode::Double);
                                }
                            }
                            _ => current.push_char('\\', QuoteMode::Double),
                        }
                    } else if let Some(next) = chars.next() {
                        current.push_char(next, QuoteMode::Single);
                    }
                }
                '\'' if !in_double_quote => {
                    in_single_quote = true;
                    // Гарантируем сохранение пустого токена `""` или `''`
                    if current.is_empty() {
                        current.push_segment(WordSegment::Literal(String::new()));
                    }
                }
                '"' => {
                    in_double_quote = !in_double_quote;
                    if current.is_empty() {
                        current.push_segment(WordSegment::DoubleQuoted(String::new()));
                    }
                }
                '#' if !in_double_quote && current.is_empty() => {
                    break;
                }
                ' ' | '\t' if !in_double_quote => {
                    flush_word(&mut words, &mut current);
                }
                '|' if !in_double_quote => {
                    flush_word(&mut words, &mut current);
                    if chars.peek() == Some(&'|') {
                        chars.next();
                        words.push(plain_op_word("||"));
                    } else {
                        words.push(plain_op_word("|"));
                    }
                }
                '&' if !in_double_quote => {
                    flush_word(&mut words, &mut current);
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        words.push(plain_op_word("&&"));
                    } else if chars.peek() == Some(&'>') {
                        chars.next();
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            words.push(plain_op_word("&>>"));
                        } else {
                            words.push(plain_op_word("&>"));
                        }
                    } else {
                        words.push(plain_op_word("&"));
                    }
                }
                ';' if !in_double_quote => {
                    flush_word(&mut words, &mut current);
                    words.push(plain_op_word(";"));
                }
                '>' | '<' if !in_double_quote => {
                    let fd_prefix = current.as_unquoted_str().filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).map(String::from);
                    if fd_prefix.is_some() {
                        current = Word::new();
                    } else {
                        flush_word(&mut words, &mut current);
                    }
                    let mut op = fd_prefix.unwrap_or_default();
                    op.push(c);
                    if c == '>' {
                        if chars.peek() == Some(&'>') {
                            chars.next();
                            op.push('>');
                        } else if chars.peek() == Some(&'&') {
                            chars.next();
                            op.push('&');
                            while let Some(&d) = chars.peek() {
                                if d.is_ascii_digit() || d == '-' {
                                    op.push(d);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    words.push(plain_op_word(&op));
                }
                _ => {
                    current.push_char(c, mode);
                }
            }
        }

        flush_word(&mut words, &mut current);
        words
    }

    /// Токенизирует строку с учетом одинарных ('...') и двойных ("...") кавычек, экранирования
    pub fn tokenize(input: &str) -> Vec<String> {
        Self::tokenize_words(input)
            .into_iter()
            .map(|w| w.raw_text())
            .collect()
    }

    /// Раскрывает переменные окружения ($VAR, $?, $USER) и подстановки $(...) в слове
    pub fn expand_word_with_sub<F, S>(
        word: &Word,
        get_var: &F,
        run_sub: &mut S,
        last_status: i32,
    ) -> String
    where
        F: Fn(&str) -> Option<String>,
        S: FnMut(&str) -> String,
    {
        let mut out = String::new();
        let is_single_unquoted = word.segments.len() == 1
            && matches!(&word.segments[0], WordSegment::Unquoted(_));
        for seg in &word.segments {
            match seg {
                WordSegment::Literal(s) => out.push_str(s),
                WordSegment::DoubleQuoted(s) => {
                    out.push_str(&Self::expand_variables_with_sub(s, get_var, run_sub, last_status));
                }
                WordSegment::Unquoted(s) => {
                    let expanded_tilde = if is_single_unquoted && (s == "~" || s.starts_with("~/")) {
                        if let Some(home) = get_var("HOME") {
                            expand_tilde(s, &home)
                        } else {
                            s.clone()
                        }
                    } else {
                        s.clone()
                    };
                    out.push_str(&Self::expand_variables_with_sub(
                        &expanded_tilde,
                        get_var,
                        run_sub,
                        last_status,
                    ));
                }
            }
        }
        out
    }

    /// Раскрывает переменные окружения ($VAR, $?, $USER) в аргументах
    pub fn expand_variables<F>(token: &str, get_var: &F, last_status: i32) -> String
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut no_sub = |_cmd: &str| String::new();
        Self::expand_variables_with_sub(token, get_var, &mut no_sub, last_status)
    }

    /// Раскрывает переменные окружения и подстановку команд `$(...)`
    pub fn expand_variables_with_sub<F, S>(
        token: &str,
        get_var: &F,
        run_sub: &mut S,
        last_status: i32,
    ) -> String
    where
        F: Fn(&str) -> Option<String>,
        S: FnMut(&str) -> String,
    {
        let mut result = String::new();
        let mut chars = token.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '`' {
                let mut inner = String::new();
                while let Some(nc) = chars.next() {
                    if nc == '\\' {
                        if let Some(esc) = chars.next() {
                            inner.push(esc);
                        }
                    } else if nc == '`' {
                        break;
                    } else {
                        inner.push(nc);
                    }
                }
                result.push_str(&run_sub(&inner));
                continue;
            }
            if c == '$' {
                if chars.peek() == Some(&'?') {
                    chars.next();
                    push_i64(&mut result, last_status as i64);
                    continue;
                }
                if matches!(chars.peek(), Some(&'#') | Some(&'@') | Some(&'*')) {
                    let key = chars.next().unwrap();
                    let mut buf = [0u8; 4];
                    let s = key.encode_utf8(&mut buf);
                    if let Some(val) = get_var(s) {
                        result.push_str(&val);
                    }
                    continue;
                }
                if chars.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    let key = chars.next().unwrap();
                    let mut buf = [0u8; 4];
                    let s = key.encode_utf8(&mut buf);
                    if let Some(val) = get_var(s) {
                        result.push_str(&val);
                    }
                    continue;
                }
                if chars.peek() == Some(&'(') {
                    let rest: String = chars.clone().collect();
                    if let Some((val, consumed)) = try_expand_arith(&rest, get_var) {
                        for _ in 0..consumed {
                            chars.next();
                        }
                        push_i64(&mut result, val);
                        continue;
                    }
                    if let Some((inner, consumed)) = try_extract_cmd_sub(&rest) {
                        for _ in 0..consumed {
                            chars.next();
                        }
                        result.push_str(&run_sub(&inner));
                        continue;
                    }
                }

                let mut var_name = String::new();
                if chars.peek() == Some(&'{') {
                    chars.next();
                    let mut brace_depth = 1usize;
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '{' {
                            brace_depth += 1;
                            var_name.push(ch);
                        } else if ch == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                            var_name.push(ch);
                        } else {
                            var_name.push(ch);
                        }
                    }
                    if !var_name.is_empty() {
                        result.push_str(&expand_braced_param_with_sub(&var_name, get_var, run_sub, last_status));
                        continue;
                    }
                } else {
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }

                if !var_name.is_empty() {
                    if let Some(val) = get_var(&var_name) {
                        result.push_str(&val);
                    }
                } else {
                    result.push('$');
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Разбирает список токенов в пайплайн команд
    pub fn parse_pipeline<F>(tokens: &[String], get_var: &F, last_status: i32) -> Option<Pipeline>
    where
        F: Fn(&str) -> Option<String>,
    {
        let words: Vec<Word> = tokens
            .iter()
            .map(|t| {
                let mut w = Word::new();
                w.push_segment(WordSegment::Unquoted(t.clone()));
                w
            })
            .collect();
        let mut no_sub = |_cmd: &str| String::new();
        Self::parse_pipeline_words(&words, get_var, &mut no_sub, last_status)
    }

    /// Разбирает список структурированных слов в пайплайн команд
    pub fn parse_pipeline_words<F, S>(
        tokens: &[Word],
        get_var: &F,
        run_sub: &mut S,
        last_status: i32,
    ) -> Option<Pipeline>
    where
        F: Fn(&str) -> Option<String>,
        S: FnMut(&str) -> String,
    {
        if tokens.is_empty() {
            return None;
        }

        let mut commands = Vec::new();
        let mut current_cmd: Option<SimpleCommand> = None;
        let mut iter = tokens.iter().peekable();

        while let Some(token) = iter.next() {
            if token.is_plain_operator("|") {
                if let Some(cmd) = current_cmd.take() {
                    if cmd.name.is_empty() {
                        return None;
                    }
                    commands.push(cmd);
                } else {
                    return None;
                }
            } else if let Some(op) = token.as_unquoted_str().and_then(parse_redirect_op) {
                match op {
                    RedirectOp::Dup { src_fd, dst_fd } => {
                        let cmd = current_cmd.as_mut()?;
                        cmd.redirects.push(Redirect::DupFd { src_fd, dst_fd });
                    }
                    RedirectOp::File { fd, is_input, is_append, both_out_err } => {
                        let target = iter.next()?;
                        let target_expanded = Self::expand_word_with_sub(target, get_var, run_sub, last_status);
                        let cmd = current_cmd.as_mut()?;
                        if both_out_err {
                            cmd.redirects.push(Redirect::Output {
                                fd: 1,
                                path: target_expanded,
                                append: is_append,
                            });
                            cmd.redirects.push(Redirect::DupFd { src_fd: 2, dst_fd: 1 });
                        } else if is_input {
                            cmd.redirects.push(Redirect::Input {
                                fd,
                                path: target_expanded,
                            });
                        } else {
                            cmd.redirects.push(Redirect::Output {
                                fd,
                                path: target_expanded,
                                append: is_append,
                            });
                        }
                    }
                }
            } else {
                let expanded = Self::expand_word_with_sub(token, get_var, run_sub, last_status);
                if let Some(cmd) = &mut current_cmd {
                    cmd.args.push(expanded);
                } else {
                    current_cmd = Some(SimpleCommand::new(expanded));
                }
            }
        }

        if let Some(cmd) = current_cmd.take() {
            if !cmd.name.is_empty() {
                commands.push(cmd);
            }
        }

        if commands.is_empty() {
            None
        } else {
            Some(Pipeline { commands })
        }
    }

    /// Разбирает список токенов в цепочку пайплайнов, разделенных операторами &&, ||, ;, &
    pub fn parse_chain<F>(tokens: &[String], get_var: &F, last_status: i32) -> Option<CommandChain>
    where
        F: Fn(&str) -> Option<String>,
    {
        let words: Vec<Word> = tokens
            .iter()
            .map(|t| {
                let mut w = Word::new();
                w.push_segment(WordSegment::Unquoted(t.clone()));
                w
            })
            .collect();
        let mut no_sub = |_cmd: &str| String::new();
        Self::parse_chain_words(&words, get_var, &mut no_sub, last_status)
    }

    /// Разбирает список структурированных слов в цепочку пайплайнов
    pub fn parse_chain_words<F, S>(
        tokens: &[Word],
        get_var: &F,
        run_sub: &mut S,
        last_status: i32,
    ) -> Option<CommandChain>
    where
        F: Fn(&str) -> Option<String>,
        S: FnMut(&str) -> String,
    {
        if tokens.is_empty() {
            return None;
        }

        let mut pipelines = Vec::new();
        let mut current_tokens: Vec<Word> = Vec::new();
        let mut in_double_bracket = false;

        for token in tokens {
            if current_tokens.is_empty() && token.raw_text() == "[[" {
                in_double_bracket = true;
            }
            if in_double_bracket {
                let is_close = token.raw_text() == "]]";
                current_tokens.push(token.clone());
                if is_close {
                    in_double_bracket = false;
                }
                continue;
            }
            if let Some(op_str) = token.as_unquoted_str().filter(|s| matches!(*s, "&&" | "||" | ";" | "&")) {
                if !current_tokens.is_empty() {
                    let pipe = Self::parse_pipeline_words(&current_tokens, get_var, run_sub, last_status)?;
                    let op = match op_str {
                        "&&" => ChainOp::And,
                        "||" => ChainOp::Or,
                        "&" => ChainOp::Background,
                        _ => ChainOp::Always,
                    };
                    pipelines.push(ChainedPipeline { pipeline: pipe, op });
                    current_tokens.clear();
                }
            } else {
                current_tokens.push(token.clone());
            }
        }

        if !current_tokens.is_empty() {
            let pipe = Self::parse_pipeline_words(&current_tokens, get_var, run_sub, last_status)?;
            pipelines.push(ChainedPipeline {
                pipeline: pipe,
                op: ChainOp::Always,
            });
        }

        if pipelines.is_empty() {
            None
        } else {
            Some(CommandChain { pipelines })
        }
    }
}

fn plain_op_word(op: &str) -> Word {
    let mut w = Word::new();
    w.push_segment(WordSegment::Unquoted(String::from(op)));
    w
}

fn collect_balanced_parens(
    chars: &mut core::iter::Peekable<core::str::Chars<'_>>,
    current: &mut Word,
    mode: QuoteMode,
) {
    let mut depth = 1usize;
    let mut sub_sq = false;
    let mut sub_dq = false;
    while let Some(c) = chars.next() {
        current.push_char(c, mode);
        if sub_sq {
            if c == '\'' {
                sub_sq = false;
            }
            continue;
        }
        if c == '\\' {
            if let Some(esc) = chars.next() {
                current.push_char(esc, mode);
            }
            continue;
        }
        if c == '\'' && !sub_dq {
            sub_sq = true;
        } else if c == '"' {
            sub_dq = !sub_dq;
        } else if !sub_dq {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }
    }
}

fn parse_ansi_c_string(chars: &mut core::iter::Peekable<core::str::Chars<'_>>) -> String {
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '\'' {
            break;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('a') => out.push('\x07'),
            Some('b') => out.push('\x08'),
            Some('e') | Some('E') => out.push('\x1b'),
            Some('f') => out.push('\x0c'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('v') => out.push('\x0b'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some('x') => {
                let mut val = 0u8;
                let mut count = 0;
                while count < 2 {
                    if let Some(&h) = chars.peek() {
                        if let Some(d) = h.to_digit(16) {
                            val = val * 16 + d as u8;
                            chars.next();
                            count += 1;
                            continue;
                        }
                    }
                    break;
                }
                if count > 0 {
                    out.push(val as char);
                } else {
                    out.push_str("\\x");
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => break,
        }
    }
    out
}

enum RedirectOp {
    File { fd: u64, is_input: bool, is_append: bool, both_out_err: bool },
    Dup { src_fd: u64, dst_fd: u64 },
}

fn parse_redirect_op(s: &str) -> Option<RedirectOp> {
    if s == "&>" {
        return Some(RedirectOp::File { fd: 1, is_input: false, is_append: false, both_out_err: true });
    }
    if s == "&>>" {
        return Some(RedirectOp::File { fd: 1, is_input: false, is_append: true, both_out_err: true });
    }
    let digit_len = s.bytes().take_while(|b| b.is_ascii_digit()).count();
    let (fd, rest) = if digit_len > 0 {
        let fd = s[..digit_len].parse::<u64>().ok()?;
        (Some(fd), &s[digit_len..])
    } else {
        (None, s)
    };

    if let Some(dst_str) = rest.strip_prefix(">&") {
        let dst_fd = dst_str.parse::<u64>().ok()?;
        return Some(RedirectOp::Dup { src_fd: fd.unwrap_or(1), dst_fd });
    }
    match rest {
        ">" => Some(RedirectOp::File { fd: fd.unwrap_or(1), is_input: false, is_append: false, both_out_err: false }),
        ">>" => Some(RedirectOp::File { fd: fd.unwrap_or(1), is_input: false, is_append: true, both_out_err: false }),
        "<" => Some(RedirectOp::File { fd: fd.unwrap_or(0), is_input: true, is_append: false, both_out_err: false }),
        _ => None,
    }
}

fn try_extract_cmd_sub(rest: &str) -> Option<(String, usize)> {
    if !rest.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    let mut in_sq = false;
    let mut in_dq = false;
    let mut escape = false;
    for (byte_idx, ch) in rest.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if in_sq {
            if ch == '\'' {
                in_sq = false;
            }
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '\'' && !in_dq {
            in_sq = true;
        } else if ch == '"' {
            in_dq = !in_dq;
        } else if !in_dq {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    let inner = &rest[1..byte_idx];
                    let consumed = rest[..=byte_idx].chars().count();
                    return Some((String::from(inner), consumed));
                }
            }
        }
    }
    None
}

fn push_i64(out: &mut String, val: i64) {
    #[cfg(feature = "std")]
    out.push_str(&val.to_string());
    #[cfg(not(feature = "std"))]
    {
        if val < 0 {
            out.push('-');
            let mut buf = [0u8; 16];
            let s = format_int((-val) as i32, &mut buf);
            out.push_str(s);
        } else {
            let mut buf = [0u8; 16];
            let s = format_int(val as i32, &mut buf);
            out.push_str(s);
        }
    }
}

fn try_expand_arith<F>(rest: &str, get_var: &F) -> Option<(i64, usize)>
where
    F: Fn(&str) -> Option<String>,
{
    if !rest.starts_with("((") {
        return None;
    }
    let mut depth = 0usize;
    let mut prev_close = false;
    for (byte_idx, ch) in rest.char_indices().skip(2) {
        if ch == '(' {
            depth += 1;
            prev_close = false;
        } else if ch == ')' {
            if depth > 0 {
                depth -= 1;
                prev_close = false;
            } else if prev_close {
                let inner = &rest[2..byte_idx - 1];
                let val = crate::script_expr::eval_arith(inner, get_var);
                let consumed_chars = rest[..=byte_idx].chars().count();
                return Some((val, consumed_chars));
            } else {
                prev_close = true;
            }
        } else {
            prev_close = false;
        }
    }
    None
}

fn expand_braced_param_with_sub<F, S>(
    expr: &str,
    get_var: &F,
    run_sub: &mut S,
    last_status: i32,
) -> String
where
    F: Fn(&str) -> Option<String>,
    S: FnMut(&str) -> String,
{
    if let Some(name) = expr.strip_prefix('#') {
        let len = get_var(name).map(|v| v.len()).unwrap_or(0);
        let mut out = String::new();
        push_i64(&mut out, len as i64);
        return out;
    }
    if let Some((name, default)) = expr.split_once(":-") {
        return match get_var(name) {
            Some(v) if !v.is_empty() => v,
            _ => Parser::expand_variables_with_sub(default, get_var, run_sub, last_status),
        };
    }
    if let Some((name, default)) = expr.split_once('-') {
        return match get_var(name) {
            Some(v) => v,
            None => Parser::expand_variables_with_sub(default, get_var, run_sub, last_status),
        };
    }
    if let Some((name, alt)) = expr.split_once(":+") {
        return match get_var(name) {
            Some(v) if !v.is_empty() => Parser::expand_variables_with_sub(alt, get_var, run_sub, last_status),
            _ => String::new(),
        };
    }
    if let Some((name, op_rest)) = split_param_pattern_op(expr) {
        let val = get_var(name).unwrap_or_default();
        return apply_param_pattern_op(&val, op_rest, get_var, run_sub, last_status);
    }
    get_var(expr).unwrap_or_default()
}

fn split_param_pattern_op(expr: &str) -> Option<(&str, &str)> {
    for (idx, ch) in expr.char_indices() {
        if idx > 0 && matches!(ch, '#' | '%' | '/') {
            return Some((&expr[..idx], &expr[idx..]));
        }
    }
    None
}

fn apply_param_pattern_op<F, S>(
    val: &str,
    op_rest: &str,
    get_var: &F,
    run_sub: &mut S,
    last_status: i32,
) -> String
where
    F: Fn(&str) -> Option<String>,
    S: FnMut(&str) -> String,
{
    if let Some(pat) = op_rest.strip_prefix("##") {
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        return strip_prefix_glob(val, &p, true);
    }
    if let Some(pat) = op_rest.strip_prefix('#') {
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        return strip_prefix_glob(val, &p, false);
    }
    if let Some(pat) = op_rest.strip_prefix("%%") {
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        return strip_suffix_glob(val, &p, true);
    }
    if let Some(pat) = op_rest.strip_prefix('%') {
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        return strip_suffix_glob(val, &p, false);
    }
    if let Some(rest) = op_rest.strip_prefix("//") {
        let (pat, rep) = rest.split_once('/').unwrap_or((rest, ""));
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        let r = Parser::expand_variables_with_sub(rep, get_var, run_sub, last_status);
        return if p.is_empty() { String::from(val) } else { val.replace(&p, &r) };
    }
    if let Some(rest) = op_rest.strip_prefix('/') {
        let (pat, rep) = rest.split_once('/').unwrap_or((rest, ""));
        let p = Parser::expand_variables_with_sub(pat, get_var, run_sub, last_status);
        let r = Parser::expand_variables_with_sub(rep, get_var, run_sub, last_status);
        return if p.is_empty() { String::from(val) } else { val.replacen(&p, &r, 1) };
    }
    String::from(val)
}

fn strip_prefix_glob(val: &str, pat: &str, longest: bool) -> String {
    let mut boundaries: Vec<usize> = val.char_indices().map(|(i, _)| i).collect();
    boundaries.push(val.len());
    if longest {
        for &i in boundaries.iter().rev() {
            if glob_match_simple(pat, &val[..i]) {
                return String::from(&val[i..]);
            }
        }
    } else {
        for &i in &boundaries {
            if glob_match_simple(pat, &val[..i]) {
                return String::from(&val[i..]);
            }
        }
    }
    String::from(val)
}

fn strip_suffix_glob(val: &str, pat: &str, longest: bool) -> String {
    let mut boundaries: Vec<usize> = val.char_indices().map(|(i, _)| i).collect();
    boundaries.push(val.len());
    if longest {
        for &i in &boundaries {
            if glob_match_simple(pat, &val[i..]) {
                return String::from(&val[..i]);
            }
        }
    } else {
        for &i in boundaries.iter().rev() {
            if glob_match_simple(pat, &val[i..]) {
                return String::from(&val[..i]);
            }
        }
    }
    String::from(val)
}

fn glob_match_simple(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star_pi, mut star_ti) = (None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(sp) = star_pi {
            pi = sp + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(not(feature = "std"))]
fn format_int<'a>(mut n: i32, buf: &'a mut [u8; 16]) -> &'a str {
    if n == 0 {
        return "0";
    }
    let is_neg = n < 0;
    if is_neg {
        n = -n;
    }
    let mut i = buf.len();
    while n > 0 && i > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    if is_neg && i > 0 {
        i -= 1;
        buf[i] = b'-';
    }
    core::str::from_utf8(&buf[i..]).unwrap_or("0")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(feature = "std")]
    use std::vec;

    #[test]
    fn test_tokenize_quotes() {
        let tokens = Parser::tokenize("echo \"hello world\" 'second arg' plain");
        assert_eq!(tokens, vec!["echo", "hello world", "second arg", "plain"]);
    }

    #[test]
    fn test_tokenize_pipeline_and_redirect() {
        let tokens = Parser::tokenize("cat file.txt | grep error > out.log");
        assert_eq!(
            tokens,
            vec!["cat", "file.txt", "|", "grep", "error", ">", "out.log"]
        );
    }

    #[test]
    fn test_expand_variables() {
        let env = |k: &str| match k {
            "USER" => Some("user".to_string()),
            "HOME" => Some("/home/user".to_string()),
            _ => None,
        };
        assert_eq!(
            Parser::expand_variables("$USER is home at $HOME", &env, 0),
            "user is home at /home/user"
        );
        assert_eq!(Parser::expand_variables("status: $?", &env, 127), "status: 127");
    }

    #[test]
    fn test_parse_pipeline() {
        let tokens = Parser::tokenize("ls -la | grep cargo > result.txt");
        let env = |_k: &str| None;
        let pipeline = match Parser::parse_pipeline(&tokens, &env, 0) {
            Some(p) => p,
            None => return,
        };

        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].name, "ls");
        assert_eq!(pipeline.commands[0].args, vec!["-la"]);
        assert_eq!(pipeline.commands[1].name, "grep");
        assert_eq!(pipeline.commands[1].args, vec!["cargo"]);
        assert_eq!(
            pipeline.commands[1].redirects,
            vec![Redirect::Output { fd: 1, path: "result.txt".to_string(), append: false }]
        );
    }

    #[test]
    fn test_tokenize_chain_operators() {
        let tokens = Parser::tokenize("cmd1 && cmd2 || cmd3 ; cmd4 & cmd5");
        assert_eq!(
            tokens,
            vec![
                "cmd1", "&&", "cmd2", "||", "cmd3", ";", "cmd4", "&", "cmd5"
            ]
        );
    }

    #[test]
    fn test_parse_chain_operators() {
        let tokens = Parser::tokenize("echo first && ls -la ; whoami");
        let env = |_k: &str| None;
        let chain = Parser::parse_chain(&tokens, &env, 0).expect("failed to parse chain");

        assert_eq!(chain.pipelines.len(), 3);
        assert_eq!(chain.pipelines[0].pipeline.commands[0].name, "echo");
        assert_eq!(chain.pipelines[0].op, ChainOp::And);

        assert_eq!(chain.pipelines[1].pipeline.commands[0].name, "ls");
        assert_eq!(chain.pipelines[1].op, ChainOp::Always);

        assert_eq!(chain.pipelines[2].pipeline.commands[0].name, "whoami");
        assert_eq!(chain.pipelines[2].op, ChainOp::Always);
    }

    #[test]
    fn test_parse_chain_background_ampersand() {
        let tokens = Parser::tokenize("sleep 10 & echo done");
        let env = |_k: &str| None;
        let chain = Parser::parse_chain(&tokens, &env, 0).expect("failed to parse chain");

        assert_eq!(chain.pipelines.len(), 2);
        assert_eq!(chain.pipelines[0].pipeline.commands[0].name, "sleep");
        assert_eq!(chain.pipelines[0].op, ChainOp::Background);

        assert_eq!(chain.pipelines[1].pipeline.commands[0].name, "echo");
        assert_eq!(chain.pipelines[1].op, ChainOp::Always);
    }

    #[test]
    fn test_tokenize_comments() {
        let tokens = Parser::tokenize("echo hello # this is a comment");
        assert_eq!(tokens, vec!["echo", "hello"]);

        let tokens_quoted = Parser::tokenize("echo \"#not_a_comment\" '#also_not'");
        assert_eq!(tokens_quoted, vec!["echo", "#not_a_comment", "#also_not"]);
    }

    #[test]
    fn test_parse_pipeline_expands_tilde() {
        let env = |k: &str| match k {
            "HOME" => Some("/home/valdis".to_string()),
            "SUB" => Some("keys".to_string()),
            _ => None,
        };
        let tokens = Parser::tokenize("cat ~ ~/.ssh/id_ed25519.pub ~user ~/$SUB > ~/out.txt");
        let pipeline = Parser::parse_pipeline(&tokens, &env, 0).expect("failed to parse pipeline");

        assert_eq!(pipeline.commands[0].name, "cat");
        assert_eq!(
            pipeline.commands[0].args,
            vec![
                "/home/valdis",
                "/home/valdis/.ssh/id_ed25519.pub",
                "~user",
                "/home/valdis/keys",
            ]
        );
        assert_eq!(
            pipeline.commands[0].redirects,
            vec![Redirect::Output { fd: 1, path: "/home/valdis/out.txt".to_string(), append: false }]
        );

        let no_home = |_k: &str| None;
        let tokens_no_home = Parser::tokenize("cat ~/.ssh/id_ed25519.pub");
        let p_no_home = Parser::parse_pipeline(&tokens_no_home, &no_home, 0).expect("failed");
        assert_eq!(p_no_home.commands[0].args, vec!["~/.ssh/id_ed25519.pub"]);
    }

    #[test]
    fn test_ansi_c_quoting_and_single_quote_protection() {
        let words = Parser::tokenize_words("RESET=$'\\e[0m' TRAP='on_error $LINENO $?'");
        assert_eq!(words.len(), 2);
        let env = |_k: &str| Some(String::from("99"));
        let mut sub = |_c: &str| String::new();
        assert_eq!(
            Parser::expand_word_with_sub(&words[0], &env, &mut sub, 0),
            "RESET=\x1b[0m"
        );
        assert_eq!(
            Parser::expand_word_with_sub(&words[1], &env, &mut sub, 1),
            "TRAP=on_error $LINENO $?"
        );
    }

    #[test]
    fn test_fd_redirections_and_cmd_sub_in_double_quotes() {
        let words = Parser::tokenize_words("printf '%s' \"$(id -un 2>/dev/null || echo 'x')\" >&2");
        let env = |_k: &str| None;
        let mut sub = |cmd: &str| format!("SUB[{}]", cmd);
        let pipe = Parser::parse_pipeline_words(&words, &env, &mut sub, 0).unwrap();
        assert_eq!(pipe.commands[0].name, "printf");
        assert_eq!(pipe.commands[0].args, vec!["%s", "SUB[id -un 2>/dev/null || echo 'x']"]);
        assert_eq!(
            pipe.commands[0].redirects,
            vec![Redirect::DupFd { src_fd: 1, dst_fd: 2 }]
        );
    }

    #[test]
    fn test_param_expansion_prefix_suffix_replace() {
        let env = |k: &str| match k {
            "x" => Some(String::from("hello_world")),
            "p" => Some(String::from("/usr/local/bin/wsh")),
            "f" => Some(String::from("archive.tar.gz")),
            "s" => Some(String::from("foo_bar_foo")),
            _ => None,
        };
        let eval = |expr: &str| {
            let mut sub = |_c: &str| String::new();
            Parser::expand_variables_with_sub(expr, &env, &mut sub, 0)
        };
        assert_eq!(eval("${x#hello_}"), "world");
        assert_eq!(eval("${p##*/}"), "wsh");
        assert_eq!(eval("${f%.*}"), "archive.tar");
        assert_eq!(eval("${f%%.*}"), "archive");
        assert_eq!(eval("${s/foo/baz}"), "baz_bar_foo");
        assert_eq!(eval("${s//foo/baz}"), "baz_bar_baz");
    }
}
