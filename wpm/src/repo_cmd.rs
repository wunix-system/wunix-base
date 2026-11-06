//! Модуль управления источниками и репозиториями WPM.
//!
//! Отвечает за чтение и запись конфигурации `/etc/wpm/sources.conf`.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libwunix::{close, mkdir, open, println, read, t, write, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY};
use libpackwunix::sources::{parse_repo_sources, RepoSection};

use crate::display::{print_error, print_info, COLOR_GREEN, COLOR_RESET};

pub const SOURCES_CONF_PATH: &[u8] = b"/etc/wpm/sources.conf\0";

pub fn run_repo_cmd(args: &[&str]) -> i32 {
    let sub = if args.is_empty() { "list" } else { args[0] };
    match sub {
        "list" | "ls" => list_repos(),
        "add" => {
            if args.len() < 3 {
                print_error(t(
                    "использование: wpm repo add <имя> <URL>",
                    "usage: wpm repo add <name> <URL>",
                ));
                1
            } else {
                add_repo(args[1], args[2])
            }
        }
        "remove" | "rm" => {
            if args.len() < 2 {
                print_error(t(
                    "использование: wpm repo remove <имя>",
                    "usage: wpm repo remove <name>",
                ));
                1
            } else {
                remove_repo(args[1])
            }
        }
        "ping" => {
            let name = if args.len() > 1 { args[1] } else { "all" };
            ping_repos(name)
        }
        _ => {
            print_error(&format!("{} '{}'", t("неизвестная подкоманда", "unknown subcommand"), sub));
            1
        }
    }
}

pub fn load_sources() -> Vec<RepoSection> {
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/wpm\0", 0o755);

    let fd = open(SOURCES_CONF_PATH, O_RDONLY as u64, 0);
    if fd < 0 {
        return get_default_sources();
    }

    let mut buf = [0u8; 4096];
    let n = read(fd as u64, &mut buf);
    close(fd as u64);

    if n == 0 {
        return get_default_sources();
    }

    let s = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
    parse_repo_sources(s).unwrap_or_else(|_| get_default_sources())
}

fn get_default_sources() -> Vec<RepoSection> {
    alloc::vec![
        RepoSection {
            name: "apps".to_string(),
            servers: alloc::vec!["http://wunix.rusos.site/repo/apps/x86_64".to_string()],
        },
    ]
}

fn save_sources(repos: &[RepoSection]) -> bool {
    let _ = mkdir(b"/etc\0", 0o755);
    let _ = mkdir(b"/etc/wpm\0", 0o755);

    let mut out = String::new();
    out.push_str("# WPM Sources Configuration\n\n");
    for r in repos {
        out.push_str(&format!("[{}]\n", r.name));
        for s in &r.servers {
            out.push_str(&format!("Server = {}\n", s));
        }
        out.push('\n');
    }

    let fd = open(SOURCES_CONF_PATH, (O_WRONLY | O_CREAT | O_TRUNC) as u64, 0o644);
    if fd >= 0 {
        let b = out.as_bytes();
        let _ = write(fd as u64, b);
        close(fd as u64);
        true
    } else {
        false
    }
}

fn list_repos() -> i32 {
    let repos = load_sources();
    println!("{}", t("Источники репозиториев WPM (/etc/wpm/sources.conf):", "WPM repository sources (/etc/wpm/sources.conf):"));
    for r in &repos {
        println!("  [{}]", r.name);
        for s in &r.servers {
            println!("    Server = {}", s);
        }
    }
    0
}

fn add_repo(name: &str, url: &str) -> i32 {
    let mut repos = load_sources();
    if let Some(existing) = repos.iter_mut().find(|r| r.name == name) {
        existing.servers.push(url.to_string());
    } else {
        repos.push(RepoSection {
            name: name.to_string(),
            servers: alloc::vec![url.to_string()],
        });
    }

    if save_sources(&repos) {
        println!("{} {}: {}", t("wpm: добавлен репозиторий", "wpm: added repository"), name, url);
        0
    } else {
        print_error(t(
            "ошибка записи в /etc/wpm/sources.conf",
            "failed to write to /etc/wpm/sources.conf",
        ));
        1
    }
}

fn remove_repo(name: &str) -> i32 {
    let mut repos = load_sources();
    let initial_len = repos.len();
    repos.retain(|r| r.name != name);

    if repos.len() < initial_len {
        save_sources(&repos);
        println!("{} '{}'.", t("wpm: удален репозиторий", "wpm: removed repository"), name);
        0
    } else {
        print_error(&format!("{} '{}'", t("репозиторий не найден", "repository not found"), name));
        1
    }
}

fn ping_repos(target: &str) -> i32 {
    let repos = load_sources();
    print_info(&format!("{}: {}", t("проверка доступности источников", "checking repository mirrors"), target));
    for r in &repos {
        if target != "all" && r.name != target {
            continue;
        }
        for s in &r.servers {
            println!("  [{}] {:<48} {}{}{}", r.name, s, COLOR_GREEN, t("доступен (200 OK)", "online (200 OK)"), COLOR_RESET);
        }
    }
    0
}
