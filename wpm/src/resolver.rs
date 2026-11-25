//! Разрешение алиасов, метапакетов и графа зависимостей WPM.

extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub fn resolve_alias(name: &str) -> &str {
    match name {
        "tcc" | "cc" => "tinycc",
        "ar" | "cpio" | "unrar" => "archiveutils",
        "unzip" => "zip",
        "gunzip" => "gzip",
        "ssh" | "sshd" | "scp" => "openssh",
        other => other,
    }
}

pub fn strip_dep_constraint(dep: &str) -> &str {
    let trimmed = dep.trim();
    for op in [">=", "<=", "==", "=", ">", "<"] {
        if let Some(idx) = trimmed.find(op) {
            return trimmed[..idx].trim();
        }
    }
    trimmed
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

/// Строит топологически отсортированный список пакетов для установки (зависимости раньше зависимых).
pub fn resolve_install_order(
    requested: &[String],
    graph: &[(String, Vec<String>)],
) -> Result<Vec<String>, &'static str> {
    let mut order: Vec<String> = Vec::new();
    let mut states: Vec<(String, VisitState)> = Vec::new();

    for req in requested {
        visit_node(req, graph, &mut states, &mut order)?;
    }

    Ok(order)
}

fn visit_node(
    name: &str,
    graph: &[(String, Vec<String>)],
    states: &mut Vec<(String, VisitState)>,
    order: &mut Vec<String>,
) -> Result<(), &'static str> {
    if let Some((_, st)) = states.iter().find(|(n, _)| n == name) {
        return match st {
            VisitState::Visited => Ok(()),
            VisitState::Visiting => Err("обнаружена циклическая зависимость пакетов"),
        };
    }

    let deps = match graph.iter().find(|(n, _)| n == name) {
        Some((_, d)) => d,
        None => return Ok(()),
    };

    states.push((name.to_string(), VisitState::Visiting));

    for raw_dep in deps {
        let dep_name = resolve_alias(strip_dep_constraint(raw_dep));
        if !dep_name.is_empty() {
            visit_node(dep_name, graph, states, order)?;
        }
    }

    if let Some((_, st)) = states.iter_mut().find(|(n, _)| n == name) {
        *st = VisitState::Visited;
    }
    order.push(name.to_string());
    Ok(())
}
