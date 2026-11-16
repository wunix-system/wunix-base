#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, format, string::String, vec, vec::Vec};

#[cfg(feature = "std")]
use std::{collections::BTreeMap, format, string::String, vec, vec::Vec};

use crate::history::History;
use crate::parser::{Parser, Word, WordSegment};
use crate::platform;
use crate::runner::execute_chain_with_ctx_opts;
use crate::script_expr::{eval_arith, glob_match};

#[derive(Debug, Clone, PartialEq)]
pub enum FlowControl {
    None,
    Break(usize),
    Continue(usize),
    Return(i32),
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub line: usize,
    pub text: String,
}

pub struct ScriptContext {
    pub script_name: String,
    pub positional: Vec<String>,
    pub functions: BTreeMap<String, Vec<Stmt>>,
    pub locals: Vec<BTreeMap<String, String>>,
    pub flow: FlowControl,
    pub errexit: bool,
    pub nounset: bool,
    pub pipefail: bool,
    pub xtrace: bool,
    pub lineno: usize,
    pub in_condition: bool,
    pub err_trap: Option<String>,
    pub exit_trap: Option<String>,
}

impl ScriptContext {
    pub fn new(script_name: &str, positional: Vec<String>) -> Self {
        Self {
            script_name: String::from(script_name),
            positional,
            functions: BTreeMap::new(),
            locals: Vec::new(),
            flow: FlowControl::None,
            errexit: false,
            nounset: false,
            pipefail: false,
            xtrace: false,
            lineno: 1,
            in_condition: false,
            err_trap: None,
            exit_trap: None,
        }
    }

    pub fn lookup_var(&self, key: &str) -> Option<String> {
        match key {
            "0" => return Some(self.script_name.clone()),
            "#" => return Some(format!("{}", self.positional.len())),
            "@" | "*" => return Some(self.positional.join(" ")),
            "LINENO" => return Some(format!("{}", self.lineno)),
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
    let stmts = split_script_stmts(content);
    let status = exec_statements(&stmts, &mut ctx, history, last_status, should_exit);
    if let Some(exit_cmd) = ctx.exit_trap.take() {
        let mut exit_dummy = false;
        let trap_stmts = split_script_stmts(&exit_cmd);
        let _ = exec_statements(&trap_stmts, &mut ctx, history, status, &mut exit_dummy);
    }
    status
}

/// Склеивает строки с `\` и разбивает скрипт на отдельные операторы по `\n` и `;`.
pub fn split_script_statements(content: &str) -> Vec<String> {
    split_script_stmts(content).into_iter().map(|s| s.text).collect()
}

pub fn split_script_stmts(content: &str) -> Vec<Stmt> {
    let joined_lines = join_continued_lines(content);
    let mut stmts = Vec::new();

    for (line_num, line) in joined_lines {
        let mut parts = Vec::new();
        split_line_by_semicolons(&line, &mut parts);
        for p in parts {
            stmts.push(Stmt { line: line_num, text: p });
        }
    }
    stmts
}

fn join_continued_lines(content: &str) -> Vec<(usize, String)> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut start_line = 1usize;
    let mut line_no = 0usize;
    for raw in content.lines() {
        line_no += 1;
        if current.is_empty() {
            start_line = line_no;
        }
        let trimmed_end = raw.trim_end_matches('\r');
        if let Some(without_slash) = trimmed_end.strip_suffix('\\') {
            current.push_str(without_slash);
            current.push(' ');
        } else {
            current.push_str(trimmed_end);
            lines.push((start_line, core::mem::take(&mut current)));
        }
    }
    if !current.is_empty() {
        lines.push((start_line, current));
    }
    lines
}

fn split_line_by_semicolons(line: &str, out: &mut Vec<String>) {
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut paren_depth = 0usize;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                cur.push(c);
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            '\'' if !in_double && paren_depth == 0 => {
                in_single = !in_single;
                cur.push(c);
            }
            '"' if !in_single && paren_depth == 0 => {
                in_double = !in_double;
                cur.push(c);
            }
            '$' if !in_single && chars.peek() == Some(&'(') => {
                cur.push(c);
                cur.push(chars.next().unwrap());
                paren_depth += 1;
            }
            '(' if paren_depth > 0 && !in_single => {
                paren_depth += 1;
                cur.push(c);
            }
            ')' if paren_depth > 0 && !in_single => {
                paren_depth -= 1;
                cur.push(c);
            }
            '#' if !in_single && !in_double && paren_depth == 0 => {
                if cur.trim().is_empty() || cur.ends_with(char::is_whitespace) {
                    break;
                }
                cur.push(c);
            }
            ';' if !in_single && !in_double && paren_depth == 0 => {
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
    stmts: &[Stmt],
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
        let s = stmts[i].text.trim();
        ctx.lineno = stmts[i].line;
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
        if last_status != 0 && !ctx.in_condition && ctx.flow == FlowControl::None && !*should_exit {
            if let Some(trap_cmd) = ctx.err_trap.clone() {
                let saved_in_cond = ctx.in_condition;
                ctx.in_condition = true;
                let trap_stmts = split_script_stmts(&trap_cmd);
                let _ = exec_statements(&trap_stmts, ctx, history, last_status, should_exit);
                ctx.in_condition = saved_in_cond;
            }
            if ctx.errexit {
                *should_exit = true;
                break;
            }
        }
        i += 1;
    }
    last_status
}

fn try_exec_control_block(
    stmts: &[Stmt],
    i: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> Option<(usize, i32)> {
    let s = stmts[i].text.trim();
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

fn eval_cmd_sub_in_ctx(
    inner: &str,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> String {
    let trimmed = inner.trim();
    if let Some(file_expr) = trimmed.strip_prefix('<') {
        let words = Parser::tokenize_words(file_expr.trim());
        if let Some(first) = words.first() {
            let path = expand_word_in_ctx(first, ctx, history, last_status, should_exit);
            let content = platform::read_file(&path).unwrap_or_default();
            return String::from(content.trim_end_matches('\n').trim_end_matches('\r'));
        }
        return String::new();
    }
    let sub_stmts = split_script_stmts(trimmed);
    let saved_in_cond = ctx.in_condition;
    ctx.in_condition = true;
    let (_, out) = platform::capture_stdout(|| {
        exec_statements(&sub_stmts, ctx, history, last_status, should_exit)
    });
    ctx.in_condition = saved_in_cond;
    out
}

fn expand_word_in_ctx(
    word: &Word,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> String {
    let mut out = String::new();
    let is_single_unquoted = word.segments.len() == 1
        && matches!(&word.segments[0], WordSegment::Unquoted(_));
    for seg in &word.segments {
        match seg {
            WordSegment::Literal(s) => out.push_str(s),
            WordSegment::DoubleQuoted(s) => {
                out.push_str(&expand_str_in_ctx(s, ctx, history, last_status, should_exit));
            }
            WordSegment::Unquoted(s) => {
                let expanded_tilde = if is_single_unquoted && (s == "~" || s.starts_with("~/")) {
                    if let Some(home) = ctx.lookup_var("HOME") {
                        crate::parser::expand_tilde(s, &home)
                    } else {
                        s.clone()
                    }
                } else {
                    s.clone()
                };
                out.push_str(&expand_str_in_ctx(
                    &expanded_tilde,
                    ctx,
                    history,
                    last_status,
                    should_exit,
                ));
            }
        }
    }
    out
}

fn expand_str_in_ctx(
    s: &str,
    ctx: &mut ScriptContext,
    _history: &mut History,
    last_status: i32,
    _should_exit: &mut bool,
) -> String {
    let ctx_cell = core::cell::RefCell::new(ctx);

    let get_var = |k: &str| -> Option<String> { ctx_cell.borrow().lookup_var(k) };
    let mut run_sub = |cmd: &str| -> String {
        let mut c = ctx_cell.borrow_mut();
        let mut sub_hist = History::new(0);
        let mut sub_exit = false;
        eval_cmd_sub_in_ctx(cmd, &mut c, &mut sub_hist, last_status, &mut sub_exit)
    };
    Parser::expand_variables_with_sub(s, &get_var, &mut run_sub, last_status)
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

    let words = Parser::tokenize_words(stmt);
    if words.is_empty() {
        return last_status;
    }

    // Проверяем, не состоит ли строка целиком из присваиваний (A=1 B=2 ...)
    if let Some(code) = try_exec_assignments(&words, ctx, history, last_status, should_exit) {
        return code;
    }

    let cmd = expand_word_in_ctx(&words[0], ctx, history, last_status, should_exit);
    let expanded_args: Vec<String> = words[1..]
        .iter()
        .map(|w| expand_word_in_ctx(w, ctx, history, last_status, should_exit))
        .collect();

    match cmd.as_str() {
        "set" => return exec_builtin_set(&expanded_args, ctx),
        "trap" => return exec_builtin_trap(&expanded_args, ctx),
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

    let ctx_cell = core::cell::RefCell::new(ctx);

    let get_var = |k: &str| -> Option<String> { ctx_cell.borrow().lookup_var(k) };
    let mut run_sub = |sub_cmd: &str| -> String {
        let mut c = ctx_cell.borrow_mut();
        let mut sub_hist = History::new(0);
        let mut sub_exit = false;
        eval_cmd_sub_in_ctx(sub_cmd, &mut c, &mut sub_hist, last_status, &mut sub_exit)
    };
    let pipefail = ctx_cell.borrow().pipefail;
    execute_chain_with_ctx_opts(stmt, &get_var, &mut run_sub, history, last_status, should_exit, pipefail)
}

fn try_exec_assignments(
    words: &[Word],
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> Option<i32> {
    let mut assignments = Vec::new();
    for w in words {
        let raw = w.raw_text();
        let (k, _) = raw.split_once('=')?;
        if !is_valid_ident(k) {
            return None;
        }
        let expanded = expand_word_in_ctx(w, ctx, history, last_status, should_exit);
        let (_, val) = expanded.split_once('=')?;
        assignments.push((String::from(k), String::from(val)));
    }
    for (k, v) in assignments {
        ctx.set_var(&k, &v);
    }
    Some(0)
}

fn exec_builtin_set(args: &[String], ctx: &mut ScriptContext) -> i32 {
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "--" {
            ctx.positional = args[i + 1..].to_vec();
            break;
        }
        if arg == "-o" || arg == "+o" {
            let enable = arg.starts_with('-');
            if let Some(opt) = args.get(i + 1) {
                match opt.as_str() {
                    "errexit" => ctx.errexit = enable,
                    "nounset" => ctx.nounset = enable,
                    "pipefail" => ctx.pipefail = enable,
                    "xtrace" => ctx.xtrace = enable,
                    _ => {}
                }
                i += 2;
                continue;
            }
        }
        if (arg.starts_with('-') || arg.starts_with('+')) && arg.len() > 1 {
            let enable = arg.starts_with('-');
            let mut chars = arg.chars().skip(1);
            while let Some(c) = chars.next() {
                match c {
                    'e' => ctx.errexit = enable,
                    'u' => ctx.nounset = enable,
                    'x' => ctx.xtrace = enable,
                    'o' => {
                        if let Some(opt) = args.get(i + 1) {
                            match opt.as_str() {
                                "errexit" => ctx.errexit = enable,
                                "nounset" => ctx.nounset = enable,
                                "pipefail" => ctx.pipefail = enable,
                                "xtrace" => ctx.xtrace = enable,
                                _ => {}
                            }
                            i += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        i += 1;
    }
    0
}

fn exec_builtin_trap(args: &[String], ctx: &mut ScriptContext) -> i32 {
    if args.is_empty() {
        return 0;
    }
    let cmd = args[0].as_str();
    let new_val = if cmd == "-" || cmd.is_empty() {
        None
    } else {
        Some(String::from(cmd))
    };
    for sig in args.iter().skip(1) {
        match sig.as_str() {
            "ERR" | "err" => ctx.err_trap = new_val.clone(),
            "EXIT" | "exit" | "0" => ctx.exit_trap = new_val.clone(),
            _ => {}
        }
    }
    0
}

fn call_function(
    body: &[Stmt],
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
    stmts: &[Stmt],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_end(stmts, start, "if", "fi");
    let mut branches: Vec<(Option<Vec<Stmt>>, Vec<Stmt>)> = Vec::new();
    let start_line = stmts[start].line;
    let mut cur_cond: Option<Vec<Stmt>> = Some(vec![Stmt {
        line: start_line,
        text: strip_prefix_kw(&stmts[start].text, "if"),
    }]);
    let mut cur_body: Vec<Stmt> = Vec::new();
    let mut in_cond = true;
    let mut depth = 0usize;
    let mut idx = start + 1;

    while idx < end {
        let s = stmts[idx].text.trim();
        let line = stmts[idx].line;
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
                cur_cond = Some(vec![Stmt { line, text: strip_prefix_kw(s, "elif") }]);
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
                    c.push(Stmt { line, text: String::from(s) });
                }
            }
        } else {
            cur_body.push(Stmt { line, text: String::from(s) });
        }
        idx += 1;
    }
    branches.push((cur_cond, cur_body));

    for (cond_opt, body) in branches {
        let matched = match cond_opt {
            Some(cond_stmts) => {
                let saved = ctx.in_condition;
                ctx.in_condition = true;
                let res = exec_statements(&cond_stmts, ctx, history, last_status, should_exit) == 0;
                ctx.in_condition = saved;
                res
            }
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
    stmts: &[Stmt],
    start: usize,
    until_mode: bool,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let kw = if until_mode { "until" } else { "while" };
    let end = find_matching_loop_end(stmts, start);
    let mut cond_stmts = vec![Stmt {
        line: stmts[start].line,
        text: strip_prefix_kw(&stmts[start].text, kw),
    }];
    let mut body = Vec::new();
    let mut in_cond = true;
    let mut depth = 0usize;

    for s in &stmts[start + 1..end] {
        let t = s.text.trim();
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
                cond_stmts.push(Stmt { line: s.line, text: String::from(t) });
            }
        } else {
            body.push(Stmt { line: s.line, text: String::from(t) });
        }
    }

    loop {
        if *should_exit {
            break;
        }
        let saved = ctx.in_condition;
        ctx.in_condition = true;
        let cond_code = exec_statements(&cond_stmts, ctx, history, last_status, should_exit);
        ctx.in_condition = saved;
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
    stmts: &[Stmt],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    mut last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_loop_end(stmts, start);
    let header = strip_prefix_kw(&stmts[start].text, "for");
    let (var_name, raw_items) = parse_for_header(&header);
    let mut body = Vec::new();
    let mut started = false;

    for s in &stmts[start + 1..end] {
        let t = s.text.trim();
        if !started && t == "do" {
            started = true;
            continue;
        }
        if started {
            body.push(Stmt { line: s.line, text: String::from(t) });
        }
    }

    let mut items = Vec::new();
    for raw in raw_items {
        let expanded = expand_str_in_ctx(&raw, ctx, history, last_status, should_exit);
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
    stmts: &[Stmt],
    start: usize,
    ctx: &mut ScriptContext,
    history: &mut History,
    last_status: i32,
    should_exit: &mut bool,
) -> (usize, i32) {
    let end = find_matching_end(stmts, start, "case", "esac");
    let header = strip_prefix_kw(&stmts[start].text, "case");
    let toks = Parser::tokenize_words(&header);
    let word = if let Some(w) = toks.first() {
        expand_word_in_ctx(w, ctx, history, last_status, should_exit)
    } else {
        String::new()
    };

    let mut idx = start + 1;
    let mut cur_patterns: Option<Vec<String>> = None;
    let mut cur_body: Vec<Stmt> = Vec::new();

    while idx < end {
        let t = stmts[idx].text.trim();
        let line = stmts[idx].line;
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
                    cur_body.push(Stmt { line, text: String::from(rest_t) });
                }
            }
        } else {
            cur_body.push(Stmt { line, text: String::from(t) });
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
    stmts: &[Stmt],
    start: usize,
    name: &str,
    inline_rest: Option<String>,
    ctx: &mut ScriptContext,
) -> (usize, i32) {
    let mut body = Vec::new();
    let start_line = stmts[start].line;
    if let Some(rest) = inline_rest {
        if !rest.is_empty() && rest != "}" {
            body.push(Stmt { line: start_line, text: rest });
        }
        if stmts[start].text.trim().ends_with('}') {
            ctx.functions.insert(String::from(name), body);
            return (start + 1, 0);
        }
    }

    let mut depth = 1usize;
    let mut idx = start + 1;
    while idx < stmts.len() {
        let t = stmts[idx].text.trim();
        let line = stmts[idx].line;
        if t.ends_with('{') {
            depth += 1;
        } else if t == "}" {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        body.push(Stmt { line, text: String::from(t) });
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

fn find_matching_end(stmts: &[Stmt], start: usize, open_kw: &str, close_kw: &str) -> usize {
    let mut depth = 0usize;
    for (idx, s) in stmts.iter().enumerate().skip(start) {
        let t = s.text.trim();
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

fn find_matching_loop_end(stmts: &[Stmt], start: usize) -> usize {
    let mut depth = 0usize;
    for (idx, s) in stmts.iter().enumerate().skip(start) {
        let t = s.text.trim();
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

    #[test]
    fn test_bash_script_full_features() {
        let tmp_file = "/tmp/wsh_test_full_hello.txt";
        let _ = std::fs::remove_file(tmp_file);
        let script = format!(r#"
set -euo pipefail
RESET=$'\e[0m'
A="x" B="y"
ts() {{ echo "12:00:00"; }}
on_error() {{
    ERR_LINE="$1"
    ERR_CODE="$2"
}}
trap 'on_error $LINENO $?' ERR
MSG="$(ts):$A:$B"
printf '%s\n' "$MSG" > "{tmp_file}"
READ_BACK="$(<"{tmp_file}")"
if [[ -f "{tmp_file}" && "$READ_BACK" == "$MSG" ]]; then
    MATCHED=yes
else
    MATCHED=no
fi
"#, tmp_file = tmp_file);
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(&script, "full.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        assert!(!exit);
        assert_eq!(platform::get_env("RESET"), Some(String::from("\x1b[0m")));
        assert_eq!(platform::get_env("MSG"), Some(String::from("12:00:00:x:y")));
        assert_eq!(platform::get_env("READ_BACK"), Some(String::from("12:00:00:x:y")));
        assert_eq!(platform::get_env("MATCHED"), Some(String::from("yes")));
        let _ = std::fs::remove_file(tmp_file);
    }

    #[test]
    fn test_set_e_and_err_trap() {
        let script = r#"
set -e
on_err() {
    TRAPPED_LINE="$1"
    TRAPPED_CODE="$2"
}
trap 'on_err $LINENO $?' ERR
AFTER=no
false
AFTER=yes
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "err.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 1);
        assert!(exit);
        assert_eq!(platform::get_env("TRAPPED_CODE"), Some(String::from("1")));
        assert_eq!(platform::get_env("AFTER"), Some(String::from("no")));
    }

    #[test]
    fn test_exact_test_sh_on_host() {
        let tmp_home = "/tmp/wsh_test_home_dir";
        let _ = std::fs::create_dir_all(tmp_home);
        let _ = std::fs::remove_file(format!("{}/HELLO.txt", tmp_home));
        platform::set_env("HOME", tmp_home);
        platform::set_env("BASH_VERSION", "5.2.0(1)-release");

        let script = r#"#!/usr/bin/env bash
set -euo pipefail

if [[ -t 1 ]]; then
    RESET=$'\e[0m'
    BOLD=$'\e[1m'
    DIM=$'\e[2m'
    RED=$'\e[31m'
    GREEN=$'\e[32m'
    YELLOW=$'\e[33m'
    BLUE=$'\e[34m'
    CYAN=$'\e[36m'
else
    RESET="" BOLD="" DIM="" RED="" GREEN="" YELLOW="" BLUE="" CYAN=""
fi

ts()      { date '+%H:%M:%S'; }
log_info()  { printf '%s[%s]%s %s[ INFO ]%s %s\n'  "$DIM" "$(ts)" "$RESET" "$BLUE$BOLD"   "$RESET" "$*"; }
log_ok()    { printf '%s[%s]%s %s[  OK  ]%s %s\n'  "$DIM" "$(ts)" "$RESET" "$GREEN$BOLD"  "$RESET" "$*"; }
log_warn()  { printf '%s[%s]%s %s[ WARN ]%s %s\n'  "$DIM" "$(ts)" "$RESET" "$YELLOW$BOLD" "$RESET" "$*" >&2; }
log_error() { printf '%s[%s]%s %s[ FAIL ]%s %s\n'  "$DIM" "$(ts)" "$RESET" "$RED$BOLD"    "$RESET" "$*" >&2; }

on_error() {
    log_error "Скрипт аварийно завершился на строке $1 (код выхода: $2)"
    exit "$2"
}
trap 'on_error $LINENO $?' ERR

TARGET_DIR="${HOME:-}"
FILE_NAME="HELLO.txt"
CONTENT="Этот файл создан с помощью скрипта и проверки работы выполнения BASH скриптов в системе Wunix"

printf '\n%s%s╔══════════════════════════════════════════════╗%s\n' "$CYAN" "$BOLD" "$RESET"
printf '%s%s║   Проверка выполнения BASH-скриптов (Wunix)  ║%s\n'   "$CYAN" "$BOLD" "$RESET"
printf '%s%s╚══════════════════════════════════════════════╝%s\n\n' "$CYAN" "$BOLD" "$RESET"

log_info "Версия Bash: ${BASH_VERSION}"
log_info "Пользователь: $(id -un 2>/dev/null || echo 'неизвестно')"

if [[ -z "$TARGET_DIR" ]]; then
    log_error "Переменная HOME не задана"
    exit 1
fi
log_info "Домашняя директория: ${CYAN}${TARGET_DIR}${RESET}"

if [[ ! -d "$TARGET_DIR" ]]; then
    log_error "Директория не существует: $TARGET_DIR"
    exit 1
fi

if [[ ! -w "$TARGET_DIR" ]]; then
    log_error "Нет прав на запись в: $TARGET_DIR"
    exit 1
fi
log_ok "Директория доступна для записи"

FILE_PATH="${TARGET_DIR}/${FILE_NAME}"

if [[ -e "$FILE_PATH" ]]; then
    log_warn "Файл уже существует и будет перезаписан: $FILE_PATH"
fi

log_info "Создаю файл ${CYAN}${FILE_NAME}${RESET}..."
printf '%s\n' "$CONTENT" > "$FILE_PATH"

if [[ -f "$FILE_PATH" ]]; then
    log_ok "Файл создан: ${CYAN}${FILE_PATH}${RESET}"
else
    log_error "Файл не был создан"
    exit 1
fi

if [[ "$(<"$FILE_PATH")" == "$CONTENT" ]]; then
    log_ok "Содержимое файла проверено и совпадает"
else
    log_error "Содержимое файла не совпадает с ожидаемым"
    exit 1
fi

SIZE=$(wc -c < "$FILE_PATH")
log_info "Размер файла: ${SIZE} байт"

printf '\n%s%s✔ Все проверки пройдены. Bash работает в Wunix!%s\n\n' "$GREEN" "$BOLD" "$RESET"
"#;
        let mut h = History::new(10);
        let mut exit = false;
        let code = run_bash_script(script, "test.sh", &[], &mut h, 0, &mut exit);
        assert_eq!(code, 0);
        let content = std::fs::read_to_string(format!("{}/HELLO.txt", tmp_home)).unwrap();
        assert_eq!(
            content.trim(),
            "Этот файл создан с помощью скрипта и проверки работы выполнения BASH скриптов в системе Wunix"
        );
        let _ = std::fs::remove_dir_all(tmp_home);
    }

    #[test]
    fn test_pipefail_and_builtin_pipeline() {
        let mut h = History::new(10);
        let mut exit = false;
        let script = r#"
set -o pipefail
false | true
echo "PIPE=$?"
"#;
        let (code, out) = platform::capture_stdout(|| {
            run_bash_script(script, "pipefail.sh", &[], &mut h, 0, &mut exit)
        });
        assert_eq!(code, 0);
        assert!(out.contains("PIPE=1"), "expected PIPE=1, got: {:?}", out);
    }
}
