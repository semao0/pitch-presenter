fn main() {
    tauri_build::build();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_macos_clang_runtime();
    }
}

fn link_macos_clang_runtime() {
    // GGML's Metal backend uses @available checks. Rust links with -nodefaultlibs,
    // so Clang does not automatically supply their compiler-rt implementation.
    let output = std::process::Command::new("xcrun")
        .args(["--sdk", "macosx", "clang", "--print-resource-dir"])
        .output()
        .expect("could not locate the macOS Clang runtime with xcrun");
    assert!(output.status.success(), "xcrun could not locate Clang");
    let resource_dir = String::from_utf8(output.stdout).expect("invalid Clang resource directory");
    let runtime_dir = std::path::Path::new(resource_dir.trim()).join("lib/darwin");
    assert!(
        runtime_dir.join("libclang_rt.osx.a").is_file(),
        "macOS Clang runtime is missing from {}",
        runtime_dir.display()
    );
    println!("cargo:rustc-link-search=native={}", runtime_dir.display());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}
