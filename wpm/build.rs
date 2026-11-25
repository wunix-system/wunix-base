use std::path::Path;
use std::process::Command;

fn git_out(dir: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=manifest.toml");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads/main");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir_path = Path::new(&manifest_dir);
    let pkg_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("wpm");
    let repo_root = dir_path.parent().unwrap_or(dir_path);

    let commit = git_out(repo_root, &["log", "-n", "1", "--format=%h", "HEAD", "--", pkg_name])
        .or_else(|| git_out(dir_path, &["rev-parse", "--short", "HEAD"]))
        .unwrap_or_else(|| "d1d8631".to_string());
    let count = git_out(repo_root, &["rev-list", "--count", "HEAD", "--", pkg_name])
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&c| c > 0)
        .unwrap_or(1);
    let dirty = git_out(repo_root, &["status", "--porcelain", "--", pkg_name])
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let suffix = if dirty { "-dirty" } else { "" };
    let pkg_version = format!("0.1.{count}-{commit}{suffix}");

    println!("cargo:rustc-env=WUNIX_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=WUNIX_PKG_VERSION={pkg_version}");
}
