use std::process::Command;

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let hash =
        git_output(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git_output(&["status", "--porcelain"])
        .map(|status| if status.is_empty() { "" } else { "-dirty" })
        .unwrap_or("");

    println!("cargo:rustc-env=QUESTLINE_BUILD_HASH={hash}");
    println!("cargo:rustc-env=QUESTLINE_BUILD_DIRTY={dirty}");
}
