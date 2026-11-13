use std::process::Command;
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
fn main() {
    for name in ["HEAD", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&["rev-parse", "--git-path", &branch])
    {
        println!("cargo:rerun-if-changed={path}");
    }
    for path in ["runtime", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let mut revision =
        git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    if git(&["status", "--porcelain", "--untracked-files=normal"])
        .is_some_and(|status| !status.is_empty())
    {
        revision.push_str("-dirty");
    }
    println!("cargo:rustc-env=MAIKO_REVISION={revision}");
}
