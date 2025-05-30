use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    let git_short_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string()) // Trim newline
        .unwrap_or_else(|| "UNKNOWN".to_string());

    // Tell Cargo to set `GIT_SHORT_HASH` for the main compilation
    println!("cargo:rustc-env=GIT_SHORT_HASH={}", git_short_hash);
}
