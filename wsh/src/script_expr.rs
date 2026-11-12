#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::{format, string::String, vec::Vec};

use crate::builtins::Builtins;

/// Проверяет соответствие строки glob-шаблону (`*` и `?`).
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star_pi, mut star_ti) = (None, 0usize);

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

/// Вычисляет целочисленное арифметическое выражение для `$((...))` и `((...))`.
pub fn eval_arith<F>(expr: &str, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let tokens = tokenize_arith(expr);
    let mut pos = 0;
    parse_arith_or(&tokens, &mut pos, get_var)
}

#[derive(Debug, Clone, PartialEq)]
enum ArithTok {
    Num(i64),
    Ident(String),
    Op(String),
}

fn tokenize_arith(expr: &str) -> Vec<ArithTok> {
    let mut out = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(ArithTok::Num(s.parse::<i64>().unwrap_or(0)));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' || c == '$' {
            if c == '$' {
                i += 1;
            }
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            if !s.is_empty() {
                out.push(ArithTok::Ident(s));
            }
            continue;
        }
        // Двухсимвольные операторы
        if i + 1 < chars.len() {
            let two: String = [c, chars[i + 1]].iter().collect();
            if matches!(
                two.as_str(),
                "==" | "!=" | "<=" | ">=" | "&&" | "||"
            ) {
                out.push(ArithTok::Op(two));
                i += 2;
                continue;
            }
        }
        out.push(ArithTok::Op(format!("{}", c)));
        i += 1;
    }
    out
}

fn parse_arith_or<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let mut lhs = parse_arith_and(toks, pos, get_var);
    while matches!(toks.get(*pos), Some(ArithTok::Op(op)) if op == "||") {
        *pos += 1;
        let rhs = parse_arith_and(toks, pos, get_var);
        lhs = if lhs != 0 || rhs != 0 { 1 } else { 0 };
    }
    lhs
}

fn parse_arith_and<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let mut lhs = parse_arith_cmp(toks, pos, get_var);
    while matches!(toks.get(*pos), Some(ArithTok::Op(op)) if op == "&&") {
        *pos += 1;
        let rhs = parse_arith_cmp(toks, pos, get_var);
        lhs = if lhs != 0 && rhs != 0 { 1 } else { 0 };
    }
    lhs
}

fn parse_arith_cmp<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let mut lhs = parse_arith_add(toks, pos, get_var);
    while let Some(ArithTok::Op(op)) = toks.get(*pos) {
        let op = op.clone();
        if !matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") {
            break;
        }
        *pos += 1;
        let rhs = parse_arith_add(toks, pos, get_var);
        lhs = match op.as_str() {
            "==" => i64::from(lhs == rhs),
            "!=" => i64::from(lhs != rhs),
            "<" => i64::from(lhs < rhs),
            "<=" => i64::from(lhs <= rhs),
            ">" => i64::from(lhs > rhs),
            ">=" => i64::from(lhs >= rhs),
            _ => lhs,
        };
    }
    lhs
}

fn parse_arith_add<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let mut lhs = parse_arith_mul(toks, pos, get_var);
    while let Some(ArithTok::Op(op)) = toks.get(*pos) {
        let op = op.clone();
        if op != "+" && op != "-" {
            break;
        }
        *pos += 1;
        let rhs = parse_arith_mul(toks, pos, get_var);
        lhs = if op == "+" {
            lhs.wrapping_add(rhs)
        } else {
            lhs.wrapping_sub(rhs)
        };
    }
    lhs
}

fn parse_arith_mul<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    let mut lhs = parse_arith_unary(toks, pos, get_var);
    while let Some(ArithTok::Op(op)) = toks.get(*pos) {
        let op = op.clone();
        if op != "*" && op != "/" && op != "%" {
            break;
        }
        *pos += 1;
        let rhs = parse_arith_unary(toks, pos, get_var);
        lhs = match op.as_str() {
            "*" => lhs.wrapping_mul(rhs),
            "/" => if rhs != 0 { lhs / rhs } else { 0 },
            "%" => if rhs != 0 { lhs % rhs } else { 0 },
            _ => lhs,
        };
    }
    lhs
}

fn parse_arith_unary<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(ArithTok::Op(op)) = toks.get(*pos) {
        if op == "-" {
            *pos += 1;
            return -parse_arith_unary(toks, pos, get_var);
        }
        if op == "+" {
            *pos += 1;
            return parse_arith_unary(toks, pos, get_var);
        }
        if op == "!" {
            *pos += 1;
            return i64::from(parse_arith_unary(toks, pos, get_var) == 0);
        }
    }
    parse_arith_primary(toks, pos, get_var)
}

fn parse_arith_primary<F>(toks: &[ArithTok], pos: &mut usize, get_var: &F) -> i64
where
    F: Fn(&str) -> Option<String>,
{
    match toks.get(*pos) {
        Some(ArithTok::Num(n)) => {
            *pos += 1;
            *n
        }
        Some(ArithTok::Ident(name)) => {
            *pos += 1;
            get_var(name)
                .and_then(|v| v.trim().parse::<i64>().ok())
                .unwrap_or(0)
        }
        Some(ArithTok::Op(op)) if op == "(" => {
            *pos += 1;
            let val = parse_arith_or(toks, pos, get_var);
            if matches!(toks.get(*pos), Some(ArithTok::Op(c)) if c == ")") {
                *pos += 1;
            }
            val
        }
        _ => 0,
    }
}

/// Вычисляет расширенное условие bash `[[ ... ]]`. Возвращает 0 (истина) или 1 (ложь).
pub fn eval_double_bracket(args: &[String]) -> i32 {
    let slice = match args.last() {
        Some(last) if last == "]]" => &args[..args.len() - 1],
        _ => args,
    };
    if eval_db_or(slice) { 0 } else { 1 }
}

fn eval_db_or(toks: &[String]) -> bool {
    if let Some(pos) = find_top_op(toks, "||") {
        return eval_db_or(&toks[..pos]) || eval_db_or(&toks[pos + 1..]);
    }
    eval_db_and(toks)
}

fn eval_db_and(toks: &[String]) -> bool {
    if let Some(pos) = find_top_op(toks, "&&") {
        return eval_db_and(&toks[..pos]) && eval_db_and(&toks[pos + 1..]);
    }
    eval_db_atom(toks)
}

fn find_top_op(toks: &[String], op: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, t) in toks.iter().enumerate() {
        if t == "(" {
            depth += 1;
        } else if t == ")" {
            depth = depth.saturating_sub(1);
        } else if depth == 0 && t == op {
            return Some(i);
        }
    }
    None
}

fn eval_db_atom(toks: &[String]) -> bool {
    if toks.is_empty() {
        return false;
    }
    if toks[0] == "!" {
        return !eval_db_atom(&toks[1..]);
    }
    if toks.len() >= 2 && toks[0] == "(" && toks[toks.len() - 1] == ")" {
        return eval_db_or(&toks[1..toks.len() - 1]);
    }
    if toks.len() == 3 {
        let (lhs, op, rhs) = (&toks[0], toks[1].as_str(), &toks[2]);
        match op {
            "==" | "=" => return glob_match(rhs, lhs),
            "!=" => return !glob_match(rhs, lhs),
            _ => {}
        }
    }
    Builtins::builtin_test(toks) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_patterns() {
        assert!(glob_match("*.sh", "install.sh"));
        assert!(glob_match("foo?bar", "fooXbar"));
        assert!(!glob_match("*.sh", "install.rs"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_eval_arith_expressions() {
        let get_var = |k: &str| match k {
            "x" => Some(String::from("10")),
            "y" => Some(String::from("3")),
            _ => None,
        };
        assert_eq!(eval_arith("1 + 2 * 3", &get_var), 7);
        assert_eq!(eval_arith("(x + y) * 2", &get_var), 26);
        assert_eq!(eval_arith("x % y", &get_var), 1);
        assert_eq!(eval_arith("x > 5 && y < 5", &get_var), 1);
        assert_eq!(eval_arith("-x + 4", &get_var), -6);
    }

    #[test]
    fn test_eval_double_bracket_conditions() {
        let args: Vec<String> = vec!["foo.sh", "==", "*.sh", "]]"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(eval_double_bracket(&args), 0);

        let args_and: Vec<String> = vec!["-n", "abc", "&&", "abc", "!=", "xyz", "]]"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(eval_double_bracket(&args_and), 0);

        let args_fail: Vec<String> = vec!["foo.txt", "==", "*.sh", "]]"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(eval_double_bracket(&args_fail), 1);
    }
}
