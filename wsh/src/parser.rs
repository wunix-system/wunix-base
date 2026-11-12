#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, string::ToString, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirect {
    Output(String),        // > file
    AppendOutput(String),  // >> file
    Input(String),         // < file
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
    /// Токенизирует строку с учетом одинарных ('...') и двойных ("...") кавычек, экранирования
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut chars = input.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut paren_depth = 0usize;

        while let Some(c) = chars.next() {
            if paren_depth > 0 && !in_single_quote && !in_double_quote {
                if c == '(' {
                    paren_depth += 1;
                } else if c == ')' {
                    paren_depth -= 1;
                }
                current.push(c);
                continue;
            }
            match c {
                '$' if !in_single_quote && !in_double_quote && chars.peek() == Some(&'(') => {
                    current.push(c);
                    if let Some(next) = chars.next() {
                        current.push(next);
                        paren_depth += 1;
                    }
                }
                '\\' if !in_single_quote => {
                    if in_double_quote {
                        // По стандарту POSIX в двойных кавычках экранируются только '"', '\\', '$', '`'
                        match chars.peek() {
                            Some(&'"') | Some(&'\\') | Some(&'$') | Some(&'`') => {
                                if let Some(next) = chars.next() {
                                    current.push(next);
                                }
                            }
                            _ => {
                                current.push('\\');
                            }
                        }
                    } else if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                '#' if !in_single_quote && !in_double_quote && current.is_empty() => {
                    // Символ '#' в начале слова вне кавычек начинает комментарий до конца строки
                    break;
                }
                ' ' | '\t' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                }
                '|' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    if chars.peek() == Some(&'|') {
                        chars.next();
                        tokens.push("||".to_string());
                    } else {
                        tokens.push("|".to_string());
                    }
                }
                '&' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        tokens.push("&&".to_string());
                    } else {
                        tokens.push("&".to_string());
                    }
                }
                ';' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    tokens.push(";".to_string());
                }
                '>' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(">>".to_string());
                    } else {
                        tokens.push(">".to_string());
                    }
                }
                '<' if !in_single_quote && !in_double_quote => {
                    if !current.is_empty() {
                        tokens.push(current);
                        current = String::new();
                    }
                    tokens.push("<".to_string());
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }

    /// Раскрывает переменные окружения ($VAR, $?, $USER) в аргументах
    pub fn expand_variables<F>(token: &str, get_var: &F, last_status: i32) -> String
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut result = String::new();
        let mut chars = token.chars().peekable();

        while let Some(c) = chars.next() {
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
                }

                let mut var_name = String::new();
                if chars.peek() == Some(&'{') {
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        if ch == '}' {
                            chars.next();
                            break;
                        }
                        var_name.push(ch);
                        chars.next();
                    }
                    if !var_name.is_empty() {
                        result.push_str(&expand_braced_param(&var_name, get_var));
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

    /// Раскрывает тильду и переменные окружения в одном токене
    fn expand_token<F>(token: &str, get_var: &F, last_status: i32) -> String
    where
        F: Fn(&str) -> Option<String>,
    {
        let tilde_expanded = if token == "~" || token.starts_with("~/") {
            if let Some(home) = get_var("HOME") {
                expand_tilde(token, &home)
            } else {
                token.to_string()
            }
        } else {
            token.to_string()
        };
        Self::expand_variables(&tilde_expanded, get_var, last_status)
    }

    /// Разбирает список токенов в пайплайн команд
    pub fn parse_pipeline<F>(tokens: &[String], get_var: &F, last_status: i32) -> Option<Pipeline>
    where
        F: Fn(&str) -> Option<String>,
    {
        if tokens.is_empty() {
            return None;
        }

        let mut commands = Vec::new();
        let mut current_cmd: Option<SimpleCommand> = None;
        let mut iter = tokens.iter().peekable();

        while let Some(token) = iter.next() {
            if token == "|" {
                if let Some(cmd) = current_cmd.take() {
                    if cmd.name.is_empty() {
                        return None;
                    }
                    commands.push(cmd);
                } else {
                    return None;
                }
            } else if token == ">" || token == ">>" || token == "<" {
                let is_append = token == ">>";
                let is_input = token == "<";
                let target = iter.next()?;
                let target_expanded = Self::expand_token(target, get_var, last_status);

                let redirect = if is_input {
                    Redirect::Input(target_expanded)
                } else if is_append {
                    Redirect::AppendOutput(target_expanded)
                } else {
                    Redirect::Output(target_expanded)
                };

                if let Some(cmd) = &mut current_cmd {
                    cmd.redirects.push(redirect);
                } else {
                    return None;
                }
            } else {
                let expanded = Self::expand_token(token, get_var, last_status);
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
        if tokens.is_empty() {
            return None;
        }

        let mut pipelines = Vec::new();
        let mut current_tokens = Vec::new();

        for token in tokens {
            if token == "&&" || token == "||" || token == ";" || token == "&" {
                if !current_tokens.is_empty() {
                    let pipe = Self::parse_pipeline(&current_tokens, get_var, last_status)?;
                    let op = match token.as_str() {
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
            let pipe = Self::parse_pipeline(&current_tokens, get_var, last_status)?;
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

fn expand_braced_param<F>(expr: &str, get_var: &F) -> String
where
    F: Fn(&str) -> Option<String>,
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
            _ => String::from(default),
        };
    }
    if let Some((name, default)) = expr.split_once('-') {
        return get_var(name).unwrap_or_else(|| String::from(default));
    }
    get_var(expr).unwrap_or_default()
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
            vec![Redirect::Output("result.txt".to_string())]
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
            vec![Redirect::Output("/home/valdis/out.txt".to_string())]
        );

        let no_home = |_k: &str| None;
        let tokens_no_home = Parser::tokenize("cat ~/.ssh/id_ed25519.pub");
        let p_no_home = Parser::parse_pipeline(&tokens_no_home, &no_home, 0).expect("failed");
        assert_eq!(p_no_home.commands[0].args, vec!["~/.ssh/id_ed25519.pub"]);
    }
}
