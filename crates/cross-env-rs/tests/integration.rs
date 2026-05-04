// Conformance integration tests for cross-env-rs.
// Each test maps to a row in the Compatibility matrix in the root README.
// Naming: `row_NN_short_name` where NN is the matrix row number.
// Row 00 is reserved for scaffold smoke checks.

use std::path::PathBuf;
use std::sync::OnceLock;

use assert_cmd::Command;
use predicates::str::contains;

// Locates (and lazily builds) the `print-env` fixture binary from the
// `print-env-fixture` workspace member. Built once per `cargo test` run.
fn print_env_bin() -> PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root above crates/cross-env-rs")
            .to_path_buf();
        escargot::CargoBuild::new()
            .package("print-env-fixture")
            .bin("print-env")
            .manifest_path(workspace_root.join("Cargo.toml"))
            .run()
            .expect("build print-env fixture")
            .path()
            .to_path_buf()
    })
    .clone()
}

#[test]
fn row_00_smoke_print_env_fixture_runs() {
    Command::new(print_env_bin())
        .arg("FIXTURE_SMOKE")
        .env("FIXTURE_SMOKE", "ok")
        .assert()
        .success()
        .stdout(contains("FIXTURE_SMOKE=ok"));
}

#[test]
fn row_00_smoke_cross_env_passes_var_to_child() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_00=hello")
        .arg(print_env_bin())
        .arg("ROW_00")
        .assert()
        .success()
        .stdout(contains("ROW_00=hello"));
}
