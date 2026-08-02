//! E2: the shipped `netrail-api` binary must be self-describing — `--sbom`
//! prints the embedded dependency inventory and exits 0 (AUDIT_ARCH §5).

#[test]
fn netrail_api_sbom_flag_prints_inventory() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_netrail-api"))
        .arg("--sbom")
        .output()
        .expect("run netrail-api --sbom");
    assert!(output.status.success(), "exit status: {}", output.status);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("# NetRail Rust dependency inventory"));
    assert!(stdout.lines().any(|l| l.starts_with("tokio@")));
    assert!(stdout.lines().any(|l| l.starts_with("axum@")));
}

#[test]
fn netrail_api_sbom_does_not_start_server() {
    // --sbom must short-circuit before binding :7421.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_netrail-api"))
        .arg("--sbom")
        .output()
        .expect("run netrail-api --sbom");
    assert!(output.status.success());
    assert!(output.stderr.is_empty() || !String::from_utf8_lossy(&output.stderr).contains("bind"));
}
