#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

#[cfg(feature = "std")]
use std::vec::Vec;

use crate::builtins::Builtins;
use crate::history::History;
use crate::parser::{ChainOp, Parser, Pipeline, SimpleCommand};
use crate::platform;

/// Исполняет строку команд с разбором конвейеров и операторов связывания (&&, ||, ;, &).
pub fn run_command_line(
    line: &str,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    let env_lookup = |key: &str| platform::get_env(key);
    let tokens = Parser::tokenize(line);
    let chain = match Parser::parse_chain(&tokens, &env_lookup, last_status) {
        Some(c) => c,
        None => return 0,
    };

    let mut skip_cond = None;

    for item in chain.pipelines {
        if *should_exit {
            break;
        }

        if let Some(cond) = skip_cond.take() {
            match cond {
                ChainOp::And if last_status != 0 => {
                    skip_cond = Some(item.op);
                    continue;
                }
                ChainOp::Or if last_status == 0 => {
                    skip_cond = Some(item.op);
                    continue;
                }
                _ => {}
            }
        }

        last_status = execute_single_pipeline(&item.pipeline, history, last_status, should_exit);

        if item.op != ChainOp::Always {
            skip_cond = Some(item.op);
        }
    }

    last_status
}

/// Выполняет одиночный конвейер команд (|).
pub fn execute_single_pipeline(
    pipeline: &Pipeline,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    if pipeline.commands.is_empty() {
        return 0;
    }

    if pipeline.commands.len() == 1 {
        let single_cmd = &pipeline.commands[0];

        // Проверяем прямое присваивание переменной без export: NAME=VALUE
        if let Some(code) = check_variable_assignment(single_cmd) {
            return code;
        }

        // Игнорируем управляющие блоки и ключевые слова шелла в командной строке ({, }, do, done, fi, then)
        if is_script_control_or_brace(&single_cmd.name) {
            return 0;
        }

        // Если команда 'if', выполняем аргументы как тестовое условие
        if single_cmd.name == "if" {
            if single_cmd.args.is_empty() {
                return 0;
            }
            let sub_cmd = SimpleCommand {
                name: single_cmd.args[0].clone(),
                args: single_cmd.args[1..].to_vec(),
                redirects: Vec::new(),
            };
            return execute_subcommand(&sub_cmd, history, last_status, should_exit);
        }

        if Builtins::is_builtin(&single_cmd.name) {
            return execute_builtin_with_redirects(single_cmd, history, last_status, should_exit);
        }
    }

    platform::execute_pipeline(pipeline)
}

/// Выполняет команду от имени Builtin (например, из 'command' или 'if')
pub fn execute_subcommand(
    cmd: &SimpleCommand,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    if Builtins::is_builtin(&cmd.name) {
        if let Some(code) = Builtins::execute(cmd, history, last_status, should_exit) {
            return code;
        }
    }

    let pipeline = Pipeline {
        commands: vec![cmd.clone()],
    };
    platform::execute_pipeline(&pipeline)
}

/// Выполняет встроенную команду с учетом перенаправлений ввода/вывода
fn execute_builtin_with_redirects(
    cmd: &SimpleCommand,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    #[cfg(not(feature = "std"))]
    {
        let saved_in = libwunix::dup(0);
        let saved_out = libwunix::dup(1);
        apply_command_redirects(&cmd.redirects);
        let res = Builtins::execute(cmd, history, last_status, should_exit);
        if saved_in >= 0 {
            let _ = libwunix::dup2(saved_in as u64, 0);
            let _ = libwunix::close(saved_in as u64);
        }
        if saved_out >= 0 {
            let _ = libwunix::dup2(saved_out as u64, 1);
            let _ = libwunix::close(saved_out as u64);
        }
        res.unwrap_or(0)
    }
    #[cfg(feature = "std")]
    {
        Builtins::execute(cmd, history, last_status, should_exit).unwrap_or(0)
    }
}

/// Выполняет содержимое шелл-скрипта построчно в текущем контексте процесса
pub fn run_script_content(
    content: &str,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if is_script_control_or_brace(trimmed) {
            continue;
        }
        last_status = run_command_line(trimmed, history, last_status, should_exit);
        if *should_exit {
            break;
        }
    }
    last_status
}

/// Распознает управляющие конструкции и разделители блоков шелла
pub fn is_script_control_or_brace(s: &str) -> bool {
    let clean = if let Some(idx) = s.find('#') {
        s[..idx].trim()
    } else {
        s.trim()
    };

    matches!(
        clean,
        "{" | "}" | "(" | ")" | "then" | "else" | "fi" | "do" | "done" | "esac"
    ) || clean.contains("() {")
        || (clean.ends_with("()") && clean.len() > 2)
        || clean.starts_with("then ")
        || clean.starts_with("else ")
        || clean.starts_with("elif ")
}

/// Проверяет прямое присваивание переменной вида `NAME=VALUE`
fn check_variable_assignment(cmd: &SimpleCommand) -> Option<i32> {
    if !cmd.args.is_empty() || !cmd.redirects.is_empty() {
        return None;
    }

    if let Some((k, v)) = cmd.name.split_once('=') {
        if is_valid_identifier(k) {
            platform::set_env(k, v);
            return Some(0);
        }
    }

    None
}

/// Проверяет корректность имени переменной окружения
fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(not(feature = "std"))]
fn apply_command_redirects(redirects: &[crate::parser::Redirect]) {
    for red in redirects {
        match red {
            crate::parser::Redirect::Output(path) => {
                let mut p_buf = [0u8; 256];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_TRUNC) as u64;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], flags, 0o666);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 1);
                        let _ = libwunix::close(fd as u64);
                    }
                }
            }
            crate::parser::Redirect::AppendOutput(path) => {
                let mut p_buf = [0u8; 256];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let flags = (libwunix::O_CREAT | libwunix::O_WRONLY | libwunix::O_APPEND) as u64;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], flags, 0o666);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 1);
                        let _ = libwunix::close(fd as u64);
                    }
                }
            }
            crate::parser::Redirect::Input(path) => {
                let mut p_buf = [0u8; 256];
                let b = path.as_bytes();
                if b.len() < p_buf.len() {
                    p_buf[..b.len()].copy_from_slice(b);
                    p_buf[b.len()] = 0;
                    let fd = libwunix::open(&p_buf[..b.len() + 1], libwunix::O_RDONLY as u64, 0);
                    if fd >= 0 {
                        let _ = libwunix::dup2(fd as u64, 0);
                        let _ = libwunix::close(fd as u64);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_script_control_or_brace() {
        assert!(is_script_control_or_brace("{"));
        assert!(is_script_control_or_brace("}"));
        assert!(is_script_control_or_brace("{ # comment"));
        assert!(is_script_control_or_brace("then"));
        assert!(is_script_control_or_brace("then # do something"));
        assert!(is_script_control_or_brace("fi"));
        assert!(is_script_control_or_brace("do"));
        assert!(is_script_control_or_brace("done"));
        assert!(is_script_control_or_brace("nvm_has() {"));
        assert!(is_script_control_or_brace("func()"));

        assert!(!is_script_control_or_brace("echo hello"));
        assert!(!is_script_control_or_brace("export PATH=/bin"));
        assert!(!is_script_control_or_brace("curl -o- https://example.com"));
    }

    #[test]
    fn test_variable_assignment_in_runner() {
        let mut history = History::new(10);
        let mut should_exit = false;

        let code = run_command_line("TEST_VAR_ABC=hello_wunix", &mut history, 0, &mut should_exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("TEST_VAR_ABC"), Some("hello_wunix".to_string()));

        // Проверяем раскрытие в последующей команде
        let code2 = run_command_line("ANOTHER_VAR=$TEST_VAR_ABC/sub", &mut history, 0, &mut should_exit);
        assert_eq!(code2, 0);
        assert_eq!(platform::get_env("ANOTHER_VAR"), Some("hello_wunix/sub".to_string()));
    }

    #[test]
    fn test_run_script_content_with_comments_and_braces() {
        let script = r#"
# Тестовый шелл-скрипт
{
VAR_X=val_x
}
# Комментарий
VAR_Y=val_y
"#;
        let mut history = History::new(10);
        let mut should_exit = false;
        let code = run_script_content(script, &mut history, 0, &mut should_exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("VAR_X"), Some("val_x".to_string()));
        assert_eq!(platform::get_env("VAR_Y"), Some("val_y".to_string()));
    }
}

