#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};

#[cfg(feature = "std")]
use std::{string::String, vec::Vec};

use crate::builtins::Builtins;
use crate::history::History;
use crate::parser::{ChainOp, Parser, Pipeline, SimpleCommand};
use crate::platform;

/// Исполняет строку команд с разбором конвейеров и операторов связывания (&&, ||, ;, &).
pub fn run_command_line(
    line: &str,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    let env_lookup = |key: &str| platform::get_env(key);
    execute_chain_with_env(line, &env_lookup, history, last_status, should_exit)
}

/// Исполняет строку команд с пользовательским обработчиком переменных окружения/скрипта.
pub fn execute_chain_with_env<F>(
    line: &str,
    env_lookup: &F,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32
where
    F: Fn(&str) -> Option<String>,
{
    let mut no_sub = |cmd: &str| -> String {
        let mut h = History::new(10);
        let mut se = false;
        platform::capture_stdout(|| {
            crate::script::run_bash_script(cmd, "wsh", &[], &mut h, 0, &mut se)
        }).1
    };
    execute_chain_with_ctx(line, env_lookup, &mut no_sub, history, last_status, should_exit)
}

/// Исполняет строку команд с пользовательским обработчиком переменных и подстановки команд `$(...)`.
pub fn execute_chain_with_ctx<F, S>(
    line: &str,
    env_lookup: &F,
    run_sub: &mut S,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32
where
    F: Fn(&str) -> Option<String>,
    S: FnMut(&str) -> String,
{
    execute_chain_with_ctx_opts(line, env_lookup, run_sub, history, last_status, should_exit, false)
}

pub fn execute_chain_with_ctx_opts<F, S>(
    line: &str,
    env_lookup: &F,
    run_sub: &mut S,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
    pipefail: bool,
) -> i32
where
    F: Fn(&str) -> Option<String>,
    S: FnMut(&str) -> String,
{
    let tokens = Parser::tokenize_words(line);
    let chain = match Parser::parse_chain_words(&tokens, env_lookup, run_sub, last_status) {
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

        last_status = execute_single_pipeline_opts(
            &item.pipeline,
            history,
            last_status,
            should_exit,
            pipefail,
        );

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
    execute_single_pipeline_opts(pipeline, history, last_status, should_exit, false)
}

fn execute_single_pipeline_opts(
    pipeline: &Pipeline,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
    pipefail: bool,
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

    platform::execute_pipeline_with_runner(pipeline, pipefail, |cmd| {
        if Builtins::is_builtin(&cmd.name) {
            Some(execute_builtin_with_redirects(cmd, history, last_status, should_exit))
        } else {
            None
        }
    })
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
pub fn execute_builtin_with_redirects(
    cmd: &SimpleCommand,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    #[cfg(not(feature = "std"))]
    {
        let saved_in = libwunix::dup(0);
        let saved_out = libwunix::dup(1);
        let saved_err = libwunix::dup(2);
        let ok = platform::apply_redirects(&cmd.redirects);
        let res = if ok {
            Builtins::execute(cmd, history, last_status, should_exit)
        } else {
            Some(1)
        };
        if saved_in >= 0 {
            let _ = libwunix::dup2(saved_in as u64, 0);
            let _ = libwunix::close(saved_in as u64);
        }
        if saved_out >= 0 {
            let _ = libwunix::dup2(saved_out as u64, 1);
            let _ = libwunix::close(saved_out as u64);
        }
        if saved_err >= 0 {
            let _ = libwunix::dup2(saved_err as u64, 2);
            let _ = libwunix::close(saved_err as u64);
        }
        res.unwrap_or(0)
    }
    #[cfg(feature = "std")]
    {
        use std::fs::{File, OpenOptions};
        use std::os::unix::io::AsRawFd;
        if cmd.redirects.is_empty() {
            return Builtins::execute(cmd, history, last_status, should_exit).unwrap_or(0);
        }
        unsafe {
            let saved_in = libc::dup(0);
            let saved_out = libc::dup(1);
            let saved_err = libc::dup(2);
            let mut ok = true;
            for red in &cmd.redirects {
                match red {
                    crate::parser::Redirect::Output { fd, path, append } => {
                        let f = if *append {
                            OpenOptions::new().create(true).append(true).open(path)
                        } else {
                            File::create(path)
                        };
                        if let Ok(file) = f {
                            libc::dup2(file.as_raw_fd(), *fd as i32);
                        } else {
                            ok = false;
                            break;
                        }
                    }
                    crate::parser::Redirect::Input { fd, path } => {
                        if let Ok(file) = File::open(path) {
                            libc::dup2(file.as_raw_fd(), *fd as i32);
                        } else {
                            ok = false;
                            break;
                        }
                    }
                    crate::parser::Redirect::DupFd { src_fd, dst_fd } => {
                        libc::dup2(*dst_fd as i32, *src_fd as i32);
                    }
                }
            }
            let res = if ok {
                Builtins::execute(cmd, history, last_status, should_exit).unwrap_or(0)
            } else {
                1
            };
            if saved_in >= 0 {
                libc::dup2(saved_in, 0);
                libc::close(saved_in);
            }
            if saved_out >= 0 {
                libc::dup2(saved_out, 1);
                libc::close(saved_out);
            }
            if saved_err >= 0 {
                libc::dup2(saved_err, 2);
                libc::close(saved_err);
            }
            res
        }
    }
}

/// Выполняет содержимое шелл-скрипта с поддержкой управляющих конструкций bash.
pub fn run_script_content(
    content: &str,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    crate::script::run_bash_script(content, "wsh", &[], history, last_status, should_exit)
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

/// Проверяет прямое присваивание переменных вида `NAME=VALUE` (одной или нескольких)
fn check_variable_assignment(cmd: &SimpleCommand) -> Option<i32> {
    if !cmd.redirects.is_empty() {
        return None;
    }

    let mut pairs = Vec::new();
    for word in core::iter::once(&cmd.name).chain(cmd.args.iter()) {
        let (k, v) = word.split_once('=')?;
        if !is_valid_identifier(k) {
            return None;
        }
        pairs.push((k, v));
    }

    for (k, v) in pairs {
        platform::set_env(k, v);
    }
    Some(0)
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

