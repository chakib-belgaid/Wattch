use assert_cmd::Command;
use predicates::str::is_match;
use std::path::Path;

#[test]
fn fake_backend_smoke_script_prints_exact_success_line() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("scripts/smoke_fake_backend.sh");
    let mut command = Command::new(script);

    command
        .assert()
        .success()
        .stdout(is_match("^wattch smoke test passed\n$").expect("valid regex"));
}
