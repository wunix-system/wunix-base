#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[cfg(feature = "std")]
use std::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::builtins::{print_err, print_out};
use crate::completion::{
    collect_all_commands, collect_dir_completions, collect_path_completions,
    collect_variable_completions, CompletionInput, CompletionItem, CompletionKind,
};
use crate::platform;
use crate::program_specs::{
    collect_env_names, collect_users, complete_builtin_program, dedup_and_sort,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompleteAction {
    Words(Vec<String>),
    Command(String),
    Dirs,
    Files,
    Commands,
    Variables,
    Users,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompleteSpec {
    pub actions: Vec<CompleteAction>,
    pub default_fallback: bool,
    pub dirnames_fallback: bool,
    pub nospace: bool,
}

struct RegistryMutex<T> {
    lock: core::sync::atomic::AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for RegistryMutex<T> {}

impl<T> RegistryMutex<T> {
    const fn new(data: T) -> Self {
        Self {
            lock: core::sync::atomic::AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(data),
        }
    }

    fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self.lock.swap(true, core::sync::atomic::Ordering::Acquire) {
            core::hint::spin_loop();
        }
        let res = f(unsafe { &mut *self.data.get() });
        self.lock.store(false, core::sync::atomic::Ordering::Release);
        res
    }
}

struct CompletionRegistry {
    specs: BTreeMap<String, CompleteSpec>,
    probed_files: BTreeSet<String>,
}

impl CompletionRegistry {
    const fn new() -> Self {
        Self {
            specs: BTreeMap::new(),
            probed_files: BTreeSet::new(),
        }
    }
}

static REGISTRY: RegistryMutex<CompletionRegistry> =
    RegistryMutex::new(CompletionRegistry::new());

pub fn register_spec(cmd: &str, spec: CompleteSpec) {
    REGISTRY.lock(|reg| {
        reg.specs.insert(String::from(cmd), spec);
    });
}

pub fn unregister_spec(cmd: &str) -> bool {
    REGISTRY.lock(|reg| reg.specs.remove(cmd).is_some())
}

pub fn get_spec(cmd: &str) -> Option<CompleteSpec> {
    REGISTRY.lock(|reg| reg.specs.get(cmd).cloned())
}

fn split_words_simple(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut esc = false;

    for c in s.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if q == '"' && c == '\\' {
                esc = true;
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\\' => esc = true,
            '\'' | '"' => quote = Some(c),
            c if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(cur);
                    cur = String::new();
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Загружает правила автодополнения для команды из системных/пользовательских файлов (однократно на команду).
pub fn ensure_file_completion_loaded(cmd_base: &str) {
    if cmd_base.is_empty() || cmd_base.contains('/') {
        return;
    }
    let already_checked = REGISTRY.lock(|reg| {
        if reg.specs.contains_key(cmd_base) || reg.probed_files.contains(cmd_base) {
            true
        } else {
            reg.probed_files.insert(String::from(cmd_base));
            false
        }
    });
    if already_checked {
        return;
    }

    let mut search_paths = Vec::new();
    if let Some(home) = platform::get_env("HOME") {
        let h = home.trim_end_matches('/');
        if !h.is_empty() {
            search_paths.push(format!("{}/.config/wsh/completions/{}", h, cmd_base));
        }
    }
    search_paths.push(format!("/etc/wsh/completions/{}", cmd_base));
    search_paths.push(format!("/usr/share/wsh/completions/{}", cmd_base));

    for path in &search_paths {
        if let Some(content) = platform::read_file(path) {
            load_completion_file_content(cmd_base, &content);
            return;
        }
    }
}

pub fn load_completion_file_content(cmd_base: &str, content: &str) {
    let mut plain_words = Vec::new();
    let mut executed_complete = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("complete ") {
            let parts = split_words_simple(line);
            if parts.len() > 1 {
                let _ = builtin_complete(&parts[1..]);
                executed_complete = true;
            }
        } else {
            for w in split_words_simple(line) {
                plain_words.push(w);
            }
        }
    }

    if !executed_complete && !plain_words.is_empty() {
        register_spec(
            cmd_base,
            CompleteSpec {
                actions: vec![CompleteAction::Words(plain_words)],
                default_fallback: true,
                dirnames_fallback: false,
                nospace: false,
            },
        );
    }
}

/// Разворачивает обёртки (`sudo`, `time`, `exec`, `command`, `env K=V`) до целевой команды и её аргументов.
fn unwrap_command_and_args<'a>(prev_words: &'a [String]) -> (Option<&'a str>, &'a [String]) {
    let mut idx = 0;
    while idx < prev_words.len() {
        let w = prev_words[idx].as_str();
        if w.contains('=') && !w.starts_with('-') {
            idx += 1;
            continue;
        }
        if matches!(w, "sudo" | "time" | "exec" | "command" | "env") {
            idx += 1;
            while idx < prev_words.len() && prev_words[idx].starts_with('-') {
                idx += 1;
            }
            continue;
        }
        return (Some(w), &prev_words[idx + 1..]);
    }
    (None, &[])
}

fn run_external_completer(
    helper: &str,
    cmd_name: &str,
    args: &[String],
    current_token: &str,
    chars_before: &[char],
) -> Vec<CompletionItem> {
    let mut exec_args = split_words_simple(helper);
    if exec_args.is_empty() {
        return Vec::new();
    }
    let prev_word = args.last().map(|s| s.as_str()).unwrap_or("");
    exec_args.push(String::from(cmd_name));
    exec_args.push(String::from(current_token));
    exec_args.push(String::from(prev_word));

    let comp_line: String = chars_before.iter().collect();
    let comp_point = format!("{}", chars_before.len());
    let comp_cword = format!("{}", args.len() + 1);
    let extra_env = [
        ("COMP_LINE", comp_line.as_str()),
        ("COMP_POINT", comp_point.as_str()),
        ("COMP_CWORD", comp_cword.as_str()),
    ];

    let mut items = Vec::new();
    if let Some(stdout) = platform::run_capture_with_env(&exec_args, &extra_env) {
        for raw_line in stdout.lines() {
            let cand = raw_line.trim_end_matches('\r').trim();
            if cand.is_empty() || !cand.starts_with(current_token) {
                continue;
            }
            let kind = if cand.ends_with('/') {
                CompletionKind::Directory
            } else if cand.starts_with('-') {
                CompletionKind::Option
            } else {
                CompletionKind::Argument
            };
            let display_name = cand.trim_end_matches('/').to_string();
            items.push(CompletionItem {
                text: cand.to_string(),
                display_name,
                kind,
            });
        }
    }
    dedup_and_sort(&mut items);
    items
}

fn eval_complete_spec(
    spec: &CompleteSpec,
    cmd_name: &str,
    args: &[String],
    current_token: &str,
    chars_before: &[char],
    home: &str,
    env_vars: &[(String, String)],
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for action in &spec.actions {
        match action {
            CompleteAction::Words(words) => {
                for w in words {
                    if w.starts_with(current_token) {
                        let kind = if w.starts_with('-') {
                            CompletionKind::Option
                        } else {
                            CompletionKind::Argument
                        };
                        items.push(CompletionItem {
                            text: w.clone(),
                            display_name: w.clone(),
                            kind,
                        });
                    }
                }
            }
            CompleteAction::Command(helper) => {
                items.extend(run_external_completer(
                    helper,
                    cmd_name,
                    args,
                    current_token,
                    chars_before,
                ));
            }
            CompleteAction::Dirs => items.extend(collect_dir_completions(current_token, home)),
            CompleteAction::Files => items.extend(collect_path_completions(current_token, home)),
            CompleteAction::Commands => items.extend(collect_all_commands(current_token)),
            CompleteAction::Variables => items.extend(collect_env_names(current_token, env_vars)),
            CompleteAction::Users => items.extend(collect_users(current_token)),
        }
    }

    if items.is_empty() {
        if spec.dirnames_fallback {
            items.extend(collect_dir_completions(current_token, home));
        } else if spec.default_fallback {
            items.extend(collect_path_completions(current_token, home));
        }
    }

    dedup_and_sort(&mut items);
    items
}

fn try_flag_equals_path(current_token: &str, home: &str) -> Option<Vec<CompletionItem>> {
    if !current_token.starts_with('-') {
        return None;
    }
    let (flag_prefix, val_part) = current_token.split_once('=')?;
    let mut items = Vec::new();
    for mut p in collect_path_completions(val_part, home) {
        p.text = format!("{}={}", flag_prefix, p.text);
        items.push(p);
    }
    Some(items)
}

/// Главная точка входа автодополнения с поддержкой команд, переменных и аргументов программ.
pub fn complete_for_input(
    input: &CompletionInput,
    chars_before: &[char],
    home: &str,
    env_vars: &[(String, String)],
) -> Vec<CompletionItem> {
    if input.current_token.starts_with('$') {
        return collect_variable_completions(&input.current_token, env_vars);
    }

    let (cmd_opt, args) = unwrap_command_and_args(&input.prev_words);
    let Some(cmd) = cmd_opt else {
        return collect_all_commands(&input.current_token);
    };

    if let Some(eq_items) = try_flag_equals_path(&input.current_token, home) {
        return eq_items;
    }

    let cmd_base = cmd.rsplit('/').next().unwrap_or(cmd);
    ensure_file_completion_loaded(cmd_base);

    if let Some(spec) = get_spec(cmd_base).or_else(|| get_spec(cmd)) {
        return eval_complete_spec(
            &spec,
            cmd,
            args,
            &input.current_token,
            chars_before,
            home,
            env_vars,
        );
    }

    complete_builtin_program(cmd_base, args, &input.current_token, home, env_vars)
}

fn format_spec_line(cmd: &str, spec: &CompleteSpec) -> String {
    let mut parts = vec![String::from("complete")];
    if spec.default_fallback {
        parts.push(String::from("-o default"));
    }
    if spec.dirnames_fallback {
        parts.push(String::from("-o dirnames"));
    }
    if spec.nospace {
        parts.push(String::from("-o nospace"));
    }
    for act in &spec.actions {
        match act {
            CompleteAction::Words(w) => parts.push(format!("-W \"{}\"", w.join(" "))),
            CompleteAction::Command(c) => parts.push(format!("-C \"{}\"", c)),
            CompleteAction::Dirs => parts.push(String::from("-d")),
            CompleteAction::Files => parts.push(String::from("-f")),
            CompleteAction::Commands => parts.push(String::from("-c")),
            CompleteAction::Variables => parts.push(String::from("-v")),
            CompleteAction::Users => parts.push(String::from("-u")),
        }
    }
    parts.push(String::from(cmd));
    parts.join(" ")
}

fn t(ru: &'static str, en: &'static str) -> &'static str {
    #[cfg(not(feature = "std"))]
    {
        libwunix::t(ru, en)
    }
    #[cfg(feature = "std")]
    {
        if let Some(lang) = platform::get_env("LANG") {
            if lang.starts_with("en") || lang.starts_with("EN") {
                return en;
            }
        }
        ru
    }
}

/// Встроенная команда `complete`: регистрирует правила автодополнения аргументов для программ.
pub fn builtin_complete(args: &[String]) -> i32 {
    if args.is_empty() {
        return print_registered_specs(&[]);
    }

    let mut spec = CompleteSpec::default();
    let mut print_mode = false;
    let mut remove_mode = false;
    let mut targets: Vec<String> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--help" | "-h" => {
                print_out(t(
                    "Использование: complete [-W СПИСОК] [-C КОМАНДА] [-d] [-f] [-c] [-v] [-u] [-o ОПЦИЯ] [-p] [-r] [ИМЯ...]\n",
                    "Usage: complete [-W WORDLIST] [-C COMMAND] [-d] [-f] [-c] [-v] [-u] [-o OPT] [-p] [-r] [NAME...]\n",
                ));
                return 0;
            }
            "-p" => print_mode = true,
            "-r" => remove_mode = true,
            "-d" => spec.actions.push(CompleteAction::Dirs),
            "-f" => spec.actions.push(CompleteAction::Files),
            "-c" => spec.actions.push(CompleteAction::Commands),
            "-v" => spec.actions.push(CompleteAction::Variables),
            "-u" => spec.actions.push(CompleteAction::Users),
            "-W" => {
                if i + 1 >= args.len() {
                    print_err(t("wsh: complete: -W требует аргумент\n", "wsh: complete: -W requires an argument\n"));
                    return 2;
                }
                i += 1;
                spec.actions.push(CompleteAction::Words(split_words_simple(&args[i])));
            }
            "-C" => {
                if i + 1 >= args.len() {
                    print_err(t("wsh: complete: -C требует аргумент\n", "wsh: complete: -C requires an argument\n"));
                    return 2;
                }
                i += 1;
                spec.actions.push(CompleteAction::Command(args[i].clone()));
            }
            "-o" => {
                if i + 1 >= args.len() {
                    print_err(t("wsh: complete: -o требует аргумент\n", "wsh: complete: -o requires an argument\n"));
                    return 2;
                }
                i += 1;
                match args[i].as_str() {
                    "default" => spec.default_fallback = true,
                    "dirnames" => spec.dirnames_fallback = true,
                    "nospace" => spec.nospace = true,
                    other => {
                        print_err(&format!("wsh: complete: {}: {}\n", t("неизвестная опция", "invalid option"), other));
                        return 2;
                    }
                }
            }
            "--" => {
                targets.extend(args[i + 1..].iter().cloned());
                break;
            }
            _ if arg.starts_with('-') => {
                print_err(&format!("wsh: complete: {}: {}\n", t("неизвестный флаг", "invalid option"), arg));
                return 2;
            }
            _ => targets.push(args[i].clone()),
        }
        i += 1;
    }

    if print_mode {
        return print_registered_specs(&targets);
    }

    if remove_mode {
        if targets.is_empty() {
            REGISTRY.lock(|reg| reg.specs.clear());
            return 0;
        }
        let mut status = 0;
        for name in &targets {
            if !unregister_spec(name) {
                status = 1;
            }
        }
        return status;
    }

    if targets.is_empty() {
        print_err(t("wsh: complete: укажите имя команды\n", "wsh: complete: command name required\n"));
        return 2;
    }

    for name in &targets {
        register_spec(name, spec.clone());
    }
    0
}

fn print_registered_specs(targets: &[String]) -> i32 {
    let entries: Vec<(String, CompleteSpec)> = REGISTRY.lock(|reg| {
        if targets.is_empty() {
            reg.specs.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            targets
                .iter()
                .filter_map(|t| reg.specs.get(t).map(|v| (t.clone(), v.clone())))
                .collect()
        }
    });
    if !targets.is_empty() && entries.is_empty() {
        return 1;
    }
    for (cmd, spec) in entries {
        print_out(&format!("{}\n", format_spec_line(&cmd, &spec)));
    }
    0
}

/// Встроенная команда `compgen`: генерирует кандидатов автодополнения по заданным правилам.
pub fn builtin_compgen(args: &[String]) -> i32 {
    let mut spec = CompleteSpec::default();
    let mut prefix = String::new();
    let mut seen_prefix = false;
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--help" | "-h" => {
                print_out(t(
                    "Использование: compgen [-W СПИСОК] [-d] [-f] [-c] [-v] [-u] [--] [ПРЕФИКС]\n",
                    "Usage: compgen [-W WORDLIST] [-d] [-f] [-c] [-v] [-u] [--] [PREFIX]\n",
                ));
                return 0;
            }
            "-d" => spec.actions.push(CompleteAction::Dirs),
            "-f" => spec.actions.push(CompleteAction::Files),
            "-c" => spec.actions.push(CompleteAction::Commands),
            "-v" => spec.actions.push(CompleteAction::Variables),
            "-u" => spec.actions.push(CompleteAction::Users),
            "-W" => {
                if i + 1 >= args.len() {
                    return 2;
                }
                i += 1;
                spec.actions.push(CompleteAction::Words(split_words_simple(&args[i])));
            }
            "--" => {
                if let Some(p) = args.get(i + 1) {
                    prefix = p.clone();
                }
                break;
            }
            _ if arg.starts_with('-') && !seen_prefix => {
                print_err(&format!("wsh: compgen: {}: {}\n", t("неизвестный флаг", "invalid option"), arg));
                return 2;
            }
            _ => {
                prefix = args[i].clone();
                seen_prefix = true;
            }
        }
        i += 1;
    }

    let home = platform::get_env("HOME").unwrap_or_else(|| String::from("/root"));
    let env_vars = platform::get_all_env();
    let items = eval_complete_spec(&spec, "", &[], &prefix, &[], &home, &env_vars);
    if items.is_empty() {
        return 1;
    }
    for it in items {
        print_out(&format!("{}\n", it.text));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parse_completion_input;

    fn complete_str(line: &str) -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        let input = parse_completion_input(&chars);
        let envs = vec![("MY_TEST_ENV".to_string(), "1".to_string())];
        complete_for_input(&input, &chars, "/root", &envs)
            .into_iter()
            .map(|c| c.text)
            .collect()
    }

    #[test]
    fn test_builtin_program_subcommands_and_flags() {
        let sys_subs = complete_str("system st");
        assert_eq!(sys_subs, vec!["start", "status", "stop"]);

        let wpm_subs = complete_str("wpm up");
        assert_eq!(wpm_subs, vec!["update", "upgrade"]);

        let wpm_repo = complete_str("wpm repo ");
        assert_eq!(wpm_repo, vec!["add", "list", "remove", "update"]);

        let ip_subs = complete_str("ip a");
        assert_eq!(ip_subs, vec!["addr", "address"]);

        let ls_flags = complete_str("ls --h");
        assert_eq!(ls_flags, vec!["--help", "--human-readable"]);

        let sudo_sys = complete_str("sudo system st");
        assert_eq!(sudo_sys, vec!["start", "status", "stop"]);
    }

    #[test]
    fn test_export_completes_env_vars_without_dollar() {
        let vars = complete_str("export MY_TEST");
        assert_eq!(vars, vec!["MY_TEST_ENV"]);
    }

    #[test]
    fn test_complete_builtin_word_list_and_remove() {
        let rc = builtin_complete(&[
            String::from("-W"),
            String::from("build check test --release"),
            String::from("mycargo_ut"),
        ]);
        assert_eq!(rc, 0);

        assert_eq!(complete_str("mycargo_ut ch"), vec!["check"]);
        assert_eq!(complete_str("mycargo_ut --"), vec!["--release"]);

        let (p_code, p_out) = platform::capture_stdout(|| {
            builtin_complete(&[String::from("-p"), String::from("mycargo_ut")])
        });
        assert_eq!(p_code, 0);
        assert!(p_out.contains("complete -W \"build check test --release\" mycargo_ut"));

        assert_eq!(
            builtin_complete(&[String::from("-r"), String::from("mycargo_ut")]),
            0
        );
        assert!(get_spec("mycargo_ut").is_none());
    }

    #[test]
    fn test_complete_external_command_passes_comp_env_and_args() {
        let rc = builtin_complete(&[
            String::from("-C"),
            String::from("/bin/sh -c 'printf \"%s\\n\" alpha beta \"$1\" \"cword=$COMP_CWORD\"' --"),
            String::from("dynprog_ut"),
        ]);
        assert_eq!(rc, 0);

        let res = complete_str("dynprog_ut ");
        assert!(res.contains(&String::from("alpha")));
        assert!(res.contains(&String::from("beta")));
        assert!(res.contains(&String::from("dynprog_ut")));
        assert!(res.contains(&String::from("cword=1")));

        let res_filtered = complete_str("dynprog_ut al");
        assert_eq!(res_filtered, vec!["alpha"]);
        let _ = unregister_spec("dynprog_ut");
    }

    #[test]
    fn test_compgen_wordlist() {
        let (code, out) = platform::capture_stdout(|| {
            builtin_compgen(&[
                String::from("-W"),
                String::from("start stop status restart"),
                String::from("st"),
            ])
        });
        assert_eq!(code, 0);
        assert_eq!(out, "start\nstatus\nstop");
    }

    #[test]
    fn test_load_completion_file_content() {
        load_completion_file_content(
            "customtool_ut",
            "# comment\ncomplete -W \"init run --dry-run\" customtool_ut\n",
        );
        assert_eq!(complete_str("customtool_ut i"), vec!["init"]);
        assert_eq!(complete_str("customtool_ut --"), vec!["--dry-run"]);
        let _ = unregister_spec("customtool_ut");

        load_completion_file_content("plaintool_ut", "# plain args\nfoo\nbar\n--verbose\n");
        assert_eq!(complete_str("plaintool_ut f"), vec!["foo"]);
        assert_eq!(complete_str("plaintool_ut -"), vec!["--verbose"]);
        let _ = unregister_spec("plaintool_ut");
    }
}
