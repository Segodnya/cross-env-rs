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

// ---- Matrix row 1: KEY=VAL pairs ----
#[test]
fn row_01_kv_pair_passes_var() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_01=hello")
        .arg(print_env_bin())
        .arg("ROW_01")
        .assert()
        .success()
        .stdout("ROW_01=hello\n");
}

// ---- Matrix row 2: Multiple env vars before command ----
#[test]
fn row_02_multiple_kv_pairs() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .args(["A=1", "B=2", "C=3"])
        .arg(print_env_bin())
        .args(["A", "B", "C"])
        .assert()
        .success()
        .stdout("A=1\nB=2\nC=3\n");
}

// ---- Matrix row 10: Exit code propagation ----
// Уходим через cross-env-shell, чтобы `exit 42` отработало портативно (sh / cmd).
#[test]
fn row_10_exit_code_propagation() {
    Command::cargo_bin("cross-env-shell")
        .expect("cross-env-shell binary present")
        .arg("ROW_10=x")
        .arg("exit 42")
        .assert()
        .code(42);
}

// ---- Matrix row 11: Signal-killed exit (128+sig) — Unix only ----
// Windows тут возвращает «1» (см. caveat в README) — отдельный тест на это
// не нужен, поведение известно и зафиксировано.
#[cfg(unix)]
#[test]
fn row_11_signal_killed_exit_via_shell_unix() {
    Command::cargo_bin("cross-env-shell")
        .expect("cross-env-shell binary present")
        .arg("ROW_11=x")
        .arg("kill -TERM $$")
        .assert()
        .code(143); // 128 + SIGTERM (15)
}

#[cfg(unix)]
#[test]
fn row_11_signal_killed_exit_via_no_shell_unix() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_11=x")
        .arg("sh")
        .arg("-c")
        .arg("kill -TERM $$")
        .assert()
        .code(143);
}

// ---- Matrix row 15: cross-env (no shell) vs cross-env-shell ----
// no-shell: `$ROW_15` доходит до child буквально.
// shell: sh раскрывает `$ROW_15` до значения переменной перед запуском.
#[cfg(unix)]
#[test]
fn row_15_no_shell_passes_dollar_literal() {
    // print-env запрашивает ключ "$ROW_15" (буквально, не разворачивается),
    // в env его нет → <unset>.
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_15=hello")
        .arg(print_env_bin())
        .arg("$ROW_15")
        .assert()
        .success()
        .stdout("$ROW_15=<unset>\n");
}

#[cfg(unix)]
#[test]
fn row_15_shell_expands_dollar() {
    // sh раскрывает $ROW_15 → "hello", print-env запрашивает ключ "hello" → <unset>.
    let cmd = format!("{} $ROW_15", print_env_bin().display());
    Command::cargo_bin("cross-env-shell")
        .expect("cross-env-shell binary present")
        .arg("ROW_15=hello")
        .arg(cmd)
        .assert()
        .success()
        .stdout("hello=<unset>\n");
}

// ---- Matrix row 3: Empty value (`FOO=`) ----
// `KEY=` должен установить переменную в пустую строку (set-but-empty),
// что отличается от unset.
#[test]
fn row_03_empty_value_is_set_but_empty() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_03=")
        .arg(print_env_bin())
        .arg("ROW_03")
        .assert()
        .success()
        .stdout("ROW_03=\n");
}

// ---- Matrix row 4: Value contains `=` (`FOO=a=b`) ----
// split_kv должен делить по ПЕРВОМУ `=`: key=ROW_04, value=a=b=c.
#[test]
fn row_04_value_contains_equals() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_04=a=b=c")
        .arg(print_env_bin())
        .arg("ROW_04")
        .assert()
        .success()
        .stdout("ROW_04=a=b=c\n");
}
