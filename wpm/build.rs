fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let commit = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .current_dir(&dir)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "d1d8631".to_string());
    println!("cargo:rustc-env=WUNIX_GIT_COMMIT={commit}");
}
