#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};

#[cfg(feature = "std")]
use std::{collections::BTreeMap, format, string::String, vec, vec::Vec};

use crate::history::History;
use crate::parser::Parser;
use crate::platform;
use crate::runner::execute_chain_with_env;
use crate::script_expr::{eval_arith, glob_match};

#[derive(Debug, Clone, PartialEq)]
pub enum FlowControl {
    None,
    Break(usize),
    Continue(usize),
    Return(i32),
}

pub struct ScriptContext {
    pub script_name: String,
    pub positional: Vec<String>,
    pub functions: BTreeMap<String, Vec<String>>,
    pub locals: Vec<BTreeMap<String, String>>,
    pub flow: FlowControl,
}

impl ScriptContext {
    pub fn new(script_name: &str, positional: Vec<String>) -> Self {
        Self {
            script_name: String::from(script_name),
            positional,
            functions: BTreeMap::new(),
            locals: Vec::new(),
            flow: FlowControl::None,
        }
    }

    pub fn lookup_var(&self, key: &str) -> Option<String> {
        match key {
            "0" => return Some(self.script_name.clone()),
            "#" => return Some(format!("{}", self.positional.len())),
            "@" | "*" => return Some(self.positional.join(" ")),
            _ => {}
        }
        if let Ok(idx) = key.parse::<usize>() {
            if idx >= 1 {
                return Some(self.positional.get(idx - 1).cloned().unwrap_or_default());
            }
        }
        for scope in self.locals.iter().rev() {
            if let Some(val) = scope.get(key) {
                return Some(val.clone());
            }
        }
        platform::get_env(key)
    }

    pub fn set_var(&mut self, key: &str, val: &str) {
        for scope in self.locals.iter_mut().rev() {
            if scope.contains_key(key) {
                scope.insert(String::from(key), String::from(val));
                return;
            }
        }
        platform::set_env(key, val);
    }

    pub fn set_local(&mut self, key: &str, val: &str) {
        if let Some(scope) = self.locals.last_mut() {
            scope.insert(String::from(key), String::from(val));
        } else {
            platform::set_env(key, val);
        }
    }
}

/// Запускает bash-скрипт с поддержкой блоков, циклов, функций и позиционных аргументов.
pub fn run_bash_script(
    content: &str,
    script_name: &str,
    args: &[String],
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    let mut ctx = ScriptContext::new(script_name, args.to_vec());
    let stmts = split_script_statements(content);
    exec_statements(&stmts, &mut ctx, history, last_status, should_exit)
}

/// Склеивает строки с `\` и разбивает скрипт на отдельные операторы по `\n` и `;`.
pub fn split_script_statements(content: &str) -> Vec<String> {
    let joined_lines = join_continued_lines(content);
    let mut stmts = Vec::new();

    for line in joined_lines {
        split_line_by_semicolons(&line, &mut stmts);
    }
    stmts
}

fn join_continued_lines(content: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for raw in content.lines() {
        let trimmed_end = raw.trim_end_matches('\r');
        if let Some(without_slash) = trimmed_end.strip_suffix('\\') {
            current.push_str(without_slash);
            current.push(' ');
        } else {
            current.push_str(trimmed_end);
            lines.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn split_line_by_semicolons(line: &str, out: &mut Vec<String>) {
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                cur.push(c);
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
                cur.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                cur.push(c);
            }
            '#' if !in_single && !in_double => {
                if cur.trim().is_empty() || cur.ends_with(char::is_whitespace) {
                    break;
                }
                cur.push(c);
            }
            ';' if !in_single && !in_double => {
                if chars.peek() == Some(&';') {
                    chars.next();
                    let t = cur.trim();
                    if !t.is_empty() {
                        out.push(String::from(t));
                    }
                    cur.clear();
                    out.push(String::from(";;"));
                } else {
                    let t = cur.trim();
                    if !t.is_empty() {
                        out.push(String::from(t));
                    }
                    cur.clear();
                }
            }
            _ => cur.push(c),
        }
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(String::from(t));
    }
}

/// Выполняет последовательность операторов скрипта.
pub fn exec_statements(
    stmts: &[String],
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    let mut i = 0;
    while i < stmts.len() {
        if *should_exit || ctx.flow != FlowControl::None {
            break;
        }
        let s = stmts[i].trim();
        if s.is_empty() || s == "{" || s == "}" {
            i += 1;
            continue;
        }

        if let Some((next_i, status)) = try_exec_control_block(stmts, i, ctx, history, last_status, should_exit) {
            last_status = status;
            i = next_i;
            continue;
        }

        last_status = exec_single_statement(s, ctx, history, last_status, should_exit);
        i += 1;
    }
    last_status
}

fn try_exec_control_block(
    stmts: &[String],
    i: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> Option<(usize, i32)> {
    let s = stmts[i].trim();
    if s == "if" || s.starts_with("if ") {
        return Some(exec_if_block(stmts, i, ctx, history, last_status, should_exit));
    }
    if s == "while" || s.starts_with("while ") {
        return Some(exec_while_block(stmts, i, false, ctx, history, last_status, should_exit));
    }
    if s == "until" || s.starts_with("until ") {
        return Some(exec_while_block(stmts, i, true, ctx, history, last_status, should_exit));
    }
    if s.starts_with("for ") {
        return Some(exec_for_block(stmts, i, ctx, history, last_status, should_exit));
    }
    if s.starts_with("case ") {
        return Some(exec_case_block(stmts, i, ctx, history, last_status, should_exit));
    }
    if let Some((name, body_start_inline)) = parse_function_header(s) {
        return Some(define_function(stmts, i, &name, body_start_inline, ctx));
    }
    None
}

fn exec_single_statement(
    stmt: &str,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    if stmt.starts_with("((") && stmt.ends_with("))") && stmt.len() >= 4 {
        let inner = &stmt[2..stmt.len() - 2];
        let val = eval_arith(inner, &|k| ctx.lookup_var(k));
        return if val != 0 { 0 } else { 1 };
    }

    let tokens = Parser::tokenize(stmt);
    if tokens.is_empty() {
        return last_status;
    }
    let cmd = Parser::expand_variables(&tokens[0], &|k| ctx.lookup_var(k), last_status);
    let expanded_args: Vec<String> = tokens[1..]
        .iter()
        .map(|t| Parser::expand_variables(t, &|k| ctx.lookup_var(k), last_status))
        .collect();

    match cmd.as_str() {
        "break" => {
            let n = expanded_args.first().and_then(|a| a.parse::<usize>().ok()).unwrap_or(1).max(1);
            ctx.flow = FlowControl::Break(n);
            return 0;
        }
        "continue" => {
            let n = expanded_args.first().and_then(|a| a.parse::<usize>().ok()).unwrap_or(1).max(1);
            ctx.flow = FlowControl::Continue(n);
            return 0;
        }
        "return" => {
            let code = expanded_args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(last_status);
            ctx.flow = FlowControl::Return(code);
            return code;
        }
        "shift" => {
            let n = expanded_args.first().and_then(|a| a.parse::<usize>().ok()).unwrap_or(1);
            if n <= ctx.positional.len() {
                ctx.positional.drain(..n);
                return 0;
            }
            return 1;
        }
        "local" => {
            for arg in &expanded_args {
                if let Some((k, v)) = arg.split_once('=') {
                    ctx.set_local(k, v);
                } else {
                    ctx.set_local(arg, "");
                }
            }
            return 0;
        }
        _ => {}
    }

    if let Some(fn_body) = ctx.functions.get(&cmd).cloned() {
        return call_function(&fn_body, &expanded_args, ctx, history, last_status, should_exit);
    }

    // Одиночное присваивание NAME=VALUE без аргументов
    if tokens.len() == 1 {
        if let Some((k, v)) = tokens[0].split_once('=') {
            if is_valid_ident(k) {
                let val = Parser::expand_variables(v, &|key| ctx.lookup_var(key), last_status);
                ctx.set_var(k, &val);
                return 0;
            }
        }
    }

    execute_chain_with_env(stmt, &|k| ctx.lookup_var(k), history, last_status, should_exit)
}

fn call_function(
    body: &[String],
    args: &[String],
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> i32 {
    let saved_pos = core::mem::replace(&mut ctx.positional, args.to_vec());
    ctx.locals.push(BTreeMap::new());
    let mut status = exec_statements(body, ctx, history, last_status, should_exit);
    if let FlowControl::Return(code) = ctx.flow {
        status = code;
        ctx.flow = FlowControl::None;
    }
    ctx.locals.pop();
    ctx.positional = saved_pos;
    status
}

fn exec_if_block(
    stmts: &[String],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_end(stmts, start, "if", "fi");
    let mut branches: Vec<(Option<Vec<String>>, Vec<String>)> = Vec::new();
    let mut cur_cond: Option<Vec<String>> = Some(vec![strip_prefix_kw(&stmts[start], "if")]);
    let mut cur_body: Vec<String> = Vec::new();
    let mut in_cond = true;
    let mut depth = 0usize;
    let mut idx = start + 1;

    while idx < end {
        let s = stmts[idx].trim();
        if is_block_open(s, "if") {
            depth += 1;
        } else if s == "fi" && depth > 0 {
            depth -= 1;
        } else if depth == 0 {
            if in_cond && s == "then" {
                in_cond = false;
                idx += 1;
                continue;
            }
            if !in_cond && (s == "elif" || s.starts_with("elif ")) {
                branches.push((cur_cond.take(), core::mem::take(&mut cur_body)));
                cur_cond = Some(vec![strip_prefix_kw(s, "elif")]);
                in_cond = true;
                idx += 1;
                continue;
            }
            if !in_cond && s == "else" {
                branches.push((cur_cond.take(), core::mem::take(&mut cur_body)));
                cur_cond = None;
                idx += 1;
                continue;
            }
        }
        if in_cond {
            if let Some(ref mut c) = cur_cond {
                if !s.is_empty() {
                    c.push(String::from(s));
                }
            }
        } else {
            cur_body.push(String::from(s));
        }
        idx += 1;
    }
    branches.push((cur_cond, cur_body));

    for (cond_opt, body) in branches {
        let matched = match cond_opt {
            Some(cond_stmts) => exec_statements(&cond_stmts, ctx, history, last_status, should_exit) == 0,
            None => true,
        };
        if matched {
            last_status = exec_statements(&body, ctx, history, last_status, should_exit);
            return (end + 1, last_status);
        }
    }
    (end + 1, 0)
}

fn exec_while_block(
    stmts: &[String],
    start: usize,
    until_mode: bool,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let kw = if until_mode { "until" } else { "while" };
    let end = find_matching_loop_end(stmts, start);
    let mut cond_stmts = vec![strip_prefix_kw(&stmts[start], kw)];
    let mut body = Vec::new();
    let mut in_cond = true;
    let mut depth = 0usize;

    for s in &stmts[start + 1..end] {
        let t = s.trim();
        if is_loop_open(t) {
            depth += 1;
        } else if t == "done" && depth > 0 {
            depth -= 1;
        } else if depth == 0 && in_cond && t == "do" {
            in_cond = false;
            continue;
        }
        if in_cond {
            if !t.is_empty() {
                cond_stmts.push(String::from(t));
            }
        } else {
            body.push(String::from(t));
        }
    }

    loop {
        if *should_exit {
            break;
        }
        let cond_code = exec_statements(&cond_stmts, ctx, history, last_status, should_exit);
        let cond_ok = if until_mode { cond_code != 0 } else { cond_code == 0 };
        if !cond_ok || ctx.flow != FlowControl::None {
            break;
        }
        last_status = exec_statements(&body, ctx, history, last_status, should_exit);
        if handle_loop_flow(&mut ctx.flow) {
            break;
        }
    }
    (end + 1, last_status)
}

fn exec_for_block(
    stmts: &[String],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_loop_end(stmts, start);
    let header = strip_prefix_kw(&stmts[start], "for");
    let (var_name, raw_items) = parse_for_header(&header);
    let mut body = Vec::new();
    let mut started = false;

    for s in &stmts[start + 1..end] {
        let t = s.trim();
        if !started && t == "do" {
            started = true;
            continue;
        }
        if started {
            body.push(String::from(t));
        }
    }

    let mut items = Vec::new();
    for raw in raw_items {
        let expanded = Parser::expand_variables(&raw, &|k| ctx.lookup_var(k), last_status);
        for part in expanded.split_whitespace() {
            items.push(String::from(part));
        }
    }

    for item in items {
        if *should_exit {
            break;
        }
        ctx.set_var(&var_name, &item);
        last_status = exec_statements(&body, ctx, history, last_status, should_exit);
        if handle_loop_flow(&mut ctx.flow) {
            break;
        }
    }
    (end + 1, last_status)
}

fn exec_case_block(
    stmts: &[String],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_end(stmts, start, "case", "esac");
    let header = strip_prefix_kw(&stmts[start], "case");
    let toks = Parser::tokenize(&header);
    let word_raw = toks.first().map(|s| s.as_str()).unwrap_or("");
    let word = Parser::expand_variables(word_raw, &|k| ctx.lookup_var(k), last_status);

    let mut idx = start + 1;
    let mut cur_patterns: Option<Vec<String>> = None;
    let mut cur_body: Vec<String> = Vec::new();

    while idx < end {
        let t = stmts[idx].trim();
        if t == ";;" {
            if let Some(pats) = cur_patterns.take() {
                if pats.iter().any(|p| glob_match(p, &word)) {
                    let status = exec_statements(&cur_body, ctx, history, last_status, should_exit);
                    return (end + 1, status);
                }
                cur_body.clear();
            }
            idx += 1;
            continue;
        }
        if cur_patterns.is_none() {
            if let Some((pat_part, rest)) = t.split_once(')') {
                let pats: Vec<String> = pat_part.split('|').map(|p| String::from(p.trim())).collect();
                cur_patterns = Some(pats);
                let rest_t = rest.trim();
                if !rest_t.is_empty() {
                    cur_body.push(String::from(rest_t));
                }
            }
        } else {
            cur_body.push(String::from(t));
        }
        idx += 1;
    }

    if let Some(pats) = cur_patterns {
        if pats.iter().any(|p| glob_match(p, &word)) {
            let status = exec_statements(&cur_body, ctx, history, last_status, should_exit);
            return (end + 1, status);
        }
    }
    (end + 1, 0)
}

fn define_function(
    stmts: &[String],
    start: usize,
    name: &str,
    inline_rest: Option<String>,
    ctx: &mut ScriptContext,
) -> (usize, i32) {
    let mut body = Vec::new();
    if let Some(rest) = inline_rest {
        if !rest.is_empty() && rest != "}" {
            body.push(rest);
        }
        if stmts[start].trim().ends_with('}') {
            ctx.functions.insert(String::from(name), body);
            return (start + 1, 0);
        }
    }

    let mut depth = 1usize;
    let mut idx = start + 1;
    while idx < stmts.len() {
        let t = stmts[idx].trim();
        if t.ends_with('{') {
            depth += 1;
        } else if t == "}" {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        body.push(String::from(t));
        idx += 1;
    }
    ctx.functions.insert(String::from(name), body);
    (idx + 1, 0)
}

fn parse_function_header(s: &str) -> Option<(String, Option<String>)> {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("function ") {
        let rest = rest.trim();
        let (name_part, tail) = rest.split_once('{')?;
        let name = name_part.trim().trim_end_matches("()").trim();
        if is_valid_ident(name) {
            let inline = tail.trim().trim_end_matches('}').trim();
            return Some((String::from(name), Some(String::from(inline))));
        }
    }
    if let Some((before, after)) = trimmed.split_once("()") {
        let name = before.trim();
        let after_t = after.trim();
        if is_valid_ident(name) && after_t.starts_with('{') {
            let inline = after_t[1..].trim().trim_end_matches('}').trim();
            return Some((String::from(name), Some(String::from(inline))));
        }
    }
    None
}

fn handle_loop_flow(flow: &mut FlowControl) -> bool {
    match flow.clone() {
        FlowControl::Break(1) => {
            *flow = FlowControl::None;
            true
        }
        FlowControl::Break(n) if n > 1 => {
            *flow = FlowControl::Break(n - 1);
            true
        }
        FlowControl::Continue(1) => {
            *flow = FlowControl::None;
            false
        }
        FlowControl::Continue(n) if n > 1 => {
            *flow = FlowControl::Continue(n - 1);
            true
        }
        FlowControl::Return(_) => true,
        _ => false,
    }
}

fn find_matching_end(stmts: &[String], start: usize, open_kw: &str, close_kw: &str) -> usize {
    let mut depth = 0usize;
    for (idx, s) in stmts.iter().enumerate().skip(start) {
        let t = s.trim();
        if is_block_open(t, open_kw) {
            depth += 1;
        } else if t == close_kw {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return idx;
            }
        }
    }
    stmts.len().saturating_sub(1)
}

fn find_matching_loop_end(stmts: &[String], start: usize) -> usize {
    let mut depth = 0usize;
    for (idx, s) in stmts.iter().enumerate().skip(start) {
        let t = s.trim();
        if is_loop_open(t) {
            depth += 1;
        } else if t == "done" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return idx;
            }
        }
    }
    stmts.len().saturating_sub(1)
}

fn is_block_open(s: &str, kw: &str) -> bool {
    s == kw || (s.starts_with(kw) && s.as_bytes().get(kw.len()) == Some(&b' '))
}

fn is_loop_open(s: &str) -> bool {
    is_block_open(s, "for") || is_block_open(s, "while") || is_block_open(s, "until")
}

fn strip_prefix_kw(s: &str, kw: &str) -> String {
    let t = s.trim();
    if t == kw {
        String::new()
    } else {
        String::from(t[kw.len()..].trim())
    }
}

fn parse_for_header(header: &str) -> (String, Vec<String>) {
    let toks = Parser::tokenize(header);
    let var = toks.first().cloned().unwrap_or_else(|| String::from("_"));
    if toks.get(1).map(|s| s.as_str()) == Some("in") {
        (var, toks[2..].to_vec())
    } else {
        (var, Vec::new())
    }
}

fn is_valid_ident(name: &str) -> bool {
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
    fn test_if_elif_else_fi() {
        let script = r#"
X=2
if [ "$X" = "1" ]; then
    RES=one
elif [ "$X" = "2" ]; then
    RES=two
else
    RES=other
fi
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "test.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("RES"), Some(String::from("two")));
    }

    #[test]
    fn test_for_loop_and_break_continue() {
        let script = r#"
OUT=""
for item in a b c d; do
    if [ "$item" = "b" ]; then
        continue
    fi
    if [ "$item" = "d" ]; then
        break
    fi
    OUT="$OUT$item"
done
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "for.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("OUT"), Some(String::from("ac")));
    }

    #[test]
    fn test_while_loop_with_arith() {
        let script = r#"
N=0
SUM=0
while (( N < 4 )); do
    N=$((N + 1))
    SUM=$((SUM + N))
done
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "while.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("SUM"), Some(String::from("10")));
    }

    #[test]
    fn test_case_statement() {
        let script = r#"
FILE=app.rs
case "$FILE" in
    *.c|*.h)
        LANG=c
        ;;
    *.rs)
        LANG=rust
        ;;
    *)
        LANG=unknown
        ;;
esac
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "case.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        assert_eq!(platform::get_env("LANG"), Some(String::from("rust")));
    }

    #[test]
    fn test_functions_local_return_and_positional_args() {
        let script = r#"
greet() {
    local who=$1
    GREET_OUT="hi_$who"
    return 42
}
ARG0=$0
ARG1=$1
ARG_CNT=$#
shift
AFTER_SHIFT=$1
greet wunix
FN_CODE=$?
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let args = vec![String::from("first"), String::from("second")];
        run_bash_script(script, "myscript.sh", &args, &mut h, 0, &mut exit);
        assert_eq!(platform::get_env("ARG0"), Some(String::from("myscript.sh")));
        assert_eq!(platform::get_env("ARG1"), Some(String::from("first")));
        assert_eq!(platform::get_env("ARG_CNT"), Some(String::from("2")));
        assert_eq!(platform::get_env("AFTER_SHIFT"), Some(String::from("second")));
        assert_eq!(platform::get_env("GREET_OUT"), Some(String::from("hi_wunix")));
        assert_eq!(platform::get_env("FN_CODE"), Some(String::from("42")));
    }
}
