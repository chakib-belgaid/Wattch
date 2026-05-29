use assert_cmd::Command;
use predicates::str::is_match;
use std::fs;
use std::path::Path;

#[test]
fn fake_backend_smoke_script_prints_exact_success_line() {
    let script = smoke_script_path();
    let mut command = Command::new(script);

    command
        .assert()
        .success()
        .stdout(is_match("^wattch smoke test passed\n$").expect("valid regex"));
}

#[test]
fn fake_backend_smoke_script_does_not_hard_code_cargo_target_dir() {
    let script = fs::read_to_string(smoke_script_path()).expect("read smoke script");

    assert!(!script.contains("target/debug"));
}

fn smoke_script_path() -> std::path::PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("scripts/smoke_fake_backend.sh")
}
