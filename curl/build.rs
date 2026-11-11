fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let wunix_cc = std::path::Path::new(&manifest_dir).join("../../wunix/libs/libcwunix/wunix-cc");
    let cc_bin = if wunix_cc.exists() {
        wunix_cc.to_str().unwrap().to_string()
    } else {
        "wunix-cc".to_string()
    };

    let status = std::process::Command::new(cc_bin)
        .args(&["-fno-stack-protector", "-c", "src/tls.c", "-o", "src/tls.o"])
        .current_dir(&manifest_dir)
        .status();

    if let Ok(s) = status {
        if s.success() {
            let _ = std::process::Command::new("ar")
                .args(&["rcs", "libtls.a", "src/tls.o"])
                .current_dir(&manifest_dir)
                .status();
        }
    }

    if target.contains("none") {
        println!("cargo:rustc-link-search=native={}/../../wunix/libs/libcwunix/lib", manifest_dir);
        println!("cargo:rustc-link-search=native={}", manifest_dir);
        println!("cargo:rustc-link-lib=static=tls");
        println!("cargo:rustc-link-lib=static=ssl");
        println!("cargo:rustc-link-lib=static=crypto");
        println!("cargo:rustc-link-lib=static=c");
    } else {
        println!("cargo:rustc-link-search=native={}", manifest_dir);
        println!("cargo:rustc-link-lib=static=tls");
        println!("cargo:rustc-link-lib=dylib=ssl");
        println!("cargo:rustc-link-lib=dylib=crypto");
    }

    println!("cargo:rerun-if-changed=src/tls.c");
}
