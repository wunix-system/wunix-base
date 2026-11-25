#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(feature = "std")]
use std::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::completion::{
    collect_all_commands, collect_dir_completions, collect_path_completions, read_dir_entries,
    CompletionItem, CompletionKind,
};
use crate::platform;

pub(crate) fn dedup_and_sort(items: &mut Vec<CompletionItem>) {
    items.sort_by(|a, b| a.text.cmp(&b.text));
    items.dedup_by(|a, b| a.text == b.text);
}

pub(crate) fn push_matching_words(
    out: &mut Vec<CompletionItem>,
    words: &[&str],
    prefix: &str,
    kind: CompletionKind,
) {
    for &w in words {
        if w.starts_with(prefix) {
            out.push(CompletionItem {
                text: w.to_string(),
                display_name: w.to_string(),
                kind,
            });
        }
    }
}

pub(crate) fn collect_users(prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(content) = platform::read_file("/etc/passwd") {
        for line in content.lines() {
            if let Some((user, _)) = line.split_once(':') {
                if !user.is_empty() && user.starts_with(prefix) {
                    items.push(CompletionItem {
                        text: user.to_string(),
                        display_name: user.to_string(),
                        kind: CompletionKind::Argument,
                    });
                }
            }
        }
    }
    dedup_and_sort(&mut items);
    items
}

pub(crate) fn collect_env_names(prefix: &str, env_vars: &[(String, String)]) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let standard = [
        "PATH", "HOME", "USER", "SHELL", "PWD", "OLDPWD", "HOSTNAME", "TERM", "LANG", "LC_ALL",
    ];
    for &name in &standard {
        if name.starts_with(prefix) {
            items.push(CompletionItem {
                text: name.to_string(),
                display_name: name.to_string(),
                kind: CompletionKind::Variable,
            });
        }
    }
    for (k, _) in env_vars {
        if k.starts_with(prefix) {
            items.push(CompletionItem {
                text: k.clone(),
                display_name: k.clone(),
                kind: CompletionKind::Variable,
            });
        }
    }
    dedup_and_sort(&mut items);
    items
}

fn collect_services(prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for (name, _) in read_dir_entries("/etc/system.d") {
        let svc = name.strip_suffix(".service").unwrap_or(&name);
        if svc != "." && svc != ".." && svc.starts_with(prefix) {
            items.push(CompletionItem {
                text: svc.to_string(),
                display_name: svc.to_string(),
                kind: CompletionKind::Argument,
            });
        }
    }
    dedup_and_sort(&mut items);
    items
}

fn collect_wpm_packages(prefix: &str, installed_only: bool, home: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if installed_only {
        for (name, _) in read_dir_entries("/var/lib/wpm/installed") {
            let pkg = name.strip_suffix(".json").unwrap_or(&name);
            if pkg != "." && pkg != ".." && pkg.starts_with(prefix) {
                items.push(CompletionItem {
                    text: pkg.to_string(),
                    display_name: pkg.to_string(),
                    kind: CompletionKind::Argument,
                });
            }
        }
    } else {
        for repo in ["system", "apps"] {
            let idx_path = format!("/var/lib/wpm/repo/{}/x86_64/index.txt", repo);
            if let Some(content) = platform::read_file(&idx_path) {
                for line in content.lines() {
                    let pkg = line.split_whitespace().next().unwrap_or("");
                    if !pkg.is_empty() && pkg.starts_with(prefix) {
                        items.push(CompletionItem {
                            text: pkg.to_string(),
                            display_name: pkg.to_string(),
                            kind: CompletionKind::Argument,
                        });
                    }
                }
            }
        }
        for item in collect_path_completions(prefix, home) {
            if item.kind == CompletionKind::Directory || item.text.ends_with(".wp") {
                items.push(item);
            }
        }
    }
    dedup_and_sort(&mut items);
    items
}

fn collect_man_topics(prefix: &str) -> Vec<CompletionItem> {
    let mut items = collect_all_commands(prefix);
    for section in ["man1", "man2", "man3", "man5", "man8"] {
        let dir = format!("/usr/share/man/{}", section);
        for (name, _) in read_dir_entries(&dir) {
            let topic = name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&name);
            if topic != "." && topic != ".." && topic.starts_with(prefix) {
                items.push(CompletionItem {
                    text: topic.to_string(),
                    display_name: topic.to_string(),
                    kind: CompletionKind::Argument,
                });
            }
        }
    }
    dedup_and_sort(&mut items);
    items
}

fn builtin_program_flags(cmd: &str) -> &'static [&'static str] {
    match cmd {
        "ls" => &[
            "-a", "-A", "-l", "-la", "-lh", "-h", "-R", "-r", "-t", "-S", "-1", "--all",
            "--long", "--human-readable", "--help",
        ],
        "cat" => &["-n", "-b", "-s", "-E", "-T", "-A", "-h", "--help"],
        "rm" => &["-r", "-rf", "-f", "-i", "-v", "--recursive", "--force", "--help"],
        "mkdir" => &["-p", "-m", "-v", "--parents", "--help"],
        "rmdir" => &["-p", "-v", "--parents", "--help"],
        "mv" => &["-f", "-i", "-n", "-v", "--force", "--help"],
        "touch" => &["-a", "-c", "-m", "-t", "--help"],
        "ln" => &["-s", "-f", "-n", "-v", "--symbolic", "--force", "--help"],
        "readlink" => &["-f", "-e", "-m", "-n", "--canonicalize", "--help"],
        "chmod" => &["-R", "-v", "-c", "--recursive", "--help"],
        "chown" => &["-R", "-v", "-h", "--recursive", "--help"],
        "wc" => &["-l", "-w", "-c", "-m", "-L", "--lines", "--words", "--bytes", "--help"],
        "echo" => &["-n", "-e", "-E"],
        "history" => &["-c", "-d", "-w", "-r", "-a", "-h", "--help"],
        "ping" => &["-c", "-i", "-W", "-s", "-q", "-h", "--help"],
        "curl" => &[
            "-X", "-H", "-d", "-o", "-O", "-L", "-i", "-I", "-s", "-S", "-v", "-k", "-u", "-A",
            "--help", "--version",
        ],
        "uname" => &["-a", "-s", "-n", "-r", "-v", "-m", "-p", "-i", "-o", "--help", "--version"],
        "free" => &["-b", "-k", "-m", "-g", "-h", "--human", "--help"],
        "fdisk" => &["-l", "--list", "-h", "--help"],
        "mount" => &["-t", "-o", "-r", "-w", "-a", "-h", "--help"],
        "umount" => &["-f", "-l", "-a", "-h", "--help"],
        "shutdown" => &[
            "-h", "-P", "--poweroff", "-r", "--reboot", "-H", "--halt", "-c", "--cancel", "-k",
            "--no-wall", "--help", "-V", "--version",
        ],
        "reboot" => &[
            "-f", "--force", "-p", "--poweroff", "-d", "--halt", "-w", "--wtmp-only", "-n",
            "--no-sync", "--no-wall", "-h", "--help", "-V", "--version",
        ],
        "useradd" => &["-m", "-d", "-s", "-u", "-g", "-G", "-c", "-r", "-h", "--help"],
        "userdel" => &["-r", "-f", "-h", "--help"],
        "passwd" => &["-l", "-u", "-d", "-S", "-h", "--help"],
        "su" => &["-l", "-c", "-s", "-h", "--help"],
        "locale" => &["-a", "--all-locales", "set", "status", "-h", "--help"],
        "wsh" | "sh" | "bash" => &["-c", "-e", "-x", "--help"],
        "complete" => &["-W", "-C", "-d", "-f", "-c", "-v", "-u", "-o", "-p", "-r", "--help"],
        "compgen" => &["-W", "-d", "-f", "-c", "-v", "-u", "--help"],
        _ => &[],
    }
}

fn complete_system(args: &[String], cur: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let non_flag_args: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.starts_with('-'))
        .collect();

    if non_flag_args.is_empty() {
        let flags = ["-h", "--help", "-V", "--version"];
        if cur.starts_with('-') {
            push_matching_words(&mut items, &flags, cur, CompletionKind::Option);
        } else {
            let subs = [
                "start", "stop", "restart", "enable", "disable", "status", "list", "logs", "edit",
                "create",
            ];
            push_matching_words(&mut items, &subs, cur, CompletionKind::Subcommand);
            if cur.is_empty() {
                push_matching_words(&mut items, &flags, cur, CompletionKind::Option);
            }
        }
        return items;
    }

    if non_flag_args.len() == 1
        && matches!(
            non_flag_args[0],
            "start" | "stop" | "restart" | "enable" | "disable" | "status" | "logs" | "edit"
        )
    {
        return collect_services(cur);
    }

    if non_flag_args[0] == "logs" {
        push_matching_words(
            &mut items,
            &["--tail", "--head", "-f", "--follow", "-n"],
            cur,
            CompletionKind::Option,
        );
    }
    items
}

fn complete_wpm(args: &[String], cur: &str, home: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if cur.starts_with('-') {
        let flags = ["-y", "--yes", "-q", "--quiet", "--root", "-h", "--help", "-V", "--version"];
        push_matching_words(&mut items, &flags, cur, CompletionKind::Option);
        return items;
    }

    let non_flag_args: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.starts_with('-'))
        .collect();

    if non_flag_args.is_empty() {
        let subs = [
            "install", "remove", "purge", "update", "upgrade", "search", "list", "info", "show",
            "repo", "kernel", "audit", "clean", "autoremove",
        ];
        push_matching_words(&mut items, &subs, cur, CompletionKind::Subcommand);
        return items;
    }

    match non_flag_args[0] {
        "repo" if non_flag_args.len() == 1 => {
            push_matching_words(&mut items, &["list", "add", "remove", "update"], cur, CompletionKind::Subcommand);
            items
        }
        "kernel" if non_flag_args.len() == 1 => {
            push_matching_words(&mut items, &["list", "status", "upgrade", "install"], cur, CompletionKind::Subcommand);
            items
        }
        "remove" | "purge" | "info" | "show" => collect_wpm_packages(cur, true, home),
        "install" => collect_wpm_packages(cur, false, home),
        _ => collect_path_completions(cur, home),
    }
}

fn complete_ip(args: &[String], cur: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let flags = ["-br", "-brief", "-V", "--version", "-h", "--help"];
    if cur.starts_with('-') {
        push_matching_words(&mut items, &flags, cur, CompletionKind::Option);
    } else if args.iter().all(|a| a.starts_with('-')) {
        push_matching_words(&mut items, &["addr", "address", "link", "route"], cur, CompletionKind::Subcommand);
        if cur.is_empty() {
            push_matching_words(&mut items, &flags, cur, CompletionKind::Option);
        }
    }
    items
}

/// Подбирает аргументы для встроенных команд и системных утилит Wunix.
pub(crate) fn complete_builtin_program(
    cmd: &str,
    args: &[String],
    cur: &str,
    home: &str,
    env_vars: &[(String, String)],
) -> Vec<CompletionItem> {
    let mut items = match cmd {
        "cd" | "rmdir" => {
            if cur.starts_with('-') {
                let mut out = Vec::new();
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
                out
            } else {
                collect_dir_completions(cur, home)
            }
        }
        "export" | "unset" => collect_env_names(cur, env_vars),
        "which" | "type" | "help" => collect_all_commands(cur),
        "man" => collect_man_topics(cur),
        "history" | "echo" => {
            let mut out = Vec::new();
            if cur.starts_with('-') || args.is_empty() {
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
            }
            if out.is_empty() && cmd == "echo" {
                out = collect_path_completions(cur, home);
            }
            out
        }
        "system" => complete_system(args, cur),
        "wpm" => complete_wpm(args, cur, home),
        "ip" => complete_ip(args, cur),
        "timedatectl" => {
            let mut out = Vec::new();
            if cur.starts_with('-') {
                push_matching_words(&mut out, &["-h", "--help"], cur, CompletionKind::Option);
            } else if args.is_empty() {
                push_matching_words(
                    &mut out,
                    &["status", "set-time", "set-timezone", "list-timezones", "set-ntp", "timesync-status"],
                    cur,
                    CompletionKind::Subcommand,
                );
            }
            out
        }
        "shutdown" => {
            let mut out = Vec::new();
            if cur.starts_with('-') {
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
            } else {
                push_matching_words(&mut out, &["now", "+0", "+5"], cur, CompletionKind::Argument);
            }
            out
        }
        "userdel" | "passwd" | "su" => {
            if cur.starts_with('-') {
                let mut out = Vec::new();
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
                out
            } else {
                collect_users(cur)
            }
        }
        "complete" => {
            let mut out = Vec::new();
            if cur.starts_with('-') {
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
            } else if args.last().is_some_and(|s| s == "-o") {
                push_matching_words(&mut out, &["default", "dirnames", "nospace"], cur, CompletionKind::Argument);
            } else {
                out = collect_all_commands(cur);
            }
            out
        }
        _ => {
            let mut out = Vec::new();
            if cur.starts_with('-') {
                push_matching_words(&mut out, builtin_program_flags(cmd), cur, CompletionKind::Option);
            } else {
                out = collect_path_completions(cur, home);
            }
            out
        }
    };

    dedup_and_sort(&mut items);
    items
}
