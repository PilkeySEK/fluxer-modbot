use std::process::Command;

fn main() {
    // trigger recompilation when a new migration is added
    println!("cargo:rerun-if-changed=migrations");

    let git_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|output| String::from_utf8(output.stdout));
    if let Ok(Ok(hash)) = git_output {
        println!("cargo:rustc-env=GIT_HASH={}", hash.trim());
    }
}
