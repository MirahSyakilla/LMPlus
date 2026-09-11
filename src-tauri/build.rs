use std::env;
use std::fs;
use std::path::PathBuf;

fn read_trimmed_file(path: PathBuf) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));

    let release_version = env::var("LMPLUS_RELEASE_VERSION")
        .ok()
        .or_else(|| read_trimmed_file(manifest_dir.join("release-version.txt")))
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    let release_channel = env::var("LMPLUS_RELEASE_CHANNEL")
        .ok()
        .or_else(|| read_trimmed_file(manifest_dir.join("release-channel.txt")))
        .unwrap_or_else(|| "stable".to_string());

    println!("cargo:rerun-if-env-changed=LMPLUS_RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=LMPLUS_RELEASE_CHANNEL");
    println!("cargo:rerun-if-changed=release-version.txt");
    println!("cargo:rerun-if-changed=release-channel.txt");
    println!("cargo:rustc-env=LMPLUS_RELEASE_VERSION={release_version}");
    println!("cargo:rustc-env=LMPLUS_RELEASE_CHANNEL={release_channel}");

    tauri_build::build()
}
