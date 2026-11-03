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

pub struct Parser;

impl Parser {
    /// Токенизирует строку с учетом одинарных ('...') и двойных ("...") кавычек, экранирования
    pub fn tokenize(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut chars = input.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        while let Some(c) = chars.next() {
            match c {
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
                    #[cfg(feature = "std")]
                    result.push_str(&last_status.to_string());
                    #[cfg(not(feature = "std"))]
                    {
                        let mut buf = [0u8; 16];
                        let s = format_int(last_status, &mut buf);
                        result.push_str(s);
                    }
                    continue;
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
                let target_expanded = Self::expand_variables(target, get_var, last_status);

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
                let expanded = Self::expand_variables(token, get_var, last_status);
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
}
