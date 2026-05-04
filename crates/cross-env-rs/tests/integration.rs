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
// Use cross-env-shell so `exit 42` runs portably across sh / cmd.
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
// Windows returns `1` here (see README caveat); no separate Windows test
// needed — the behaviour is known and documented.
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
// no-shell: `$ROW_15` reaches the child literally.
// shell: sh expands `$ROW_15` to its value before invoking the command.
#[cfg(unix)]
#[test]
fn row_15_no_shell_passes_dollar_literal() {
    // print-env queries the literal key "$ROW_15" (no expansion);
    // it is not in env → <unset>.
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
    // sh expands $ROW_15 → "hello"; print-env then queries the key "hello" → <unset>.
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
// `KEY=` must set the variable to an empty string (set-but-empty),
// distinct from unset.
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
// split_kv must split on the FIRST `=`: key=ROW_04, value=a=b=c.
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

// ---- Matrix row 9: PATH-list separator `:` ↔ `;` auto-translate ----
// For the upstream whitelist (PATH, NODE_PATH; case-insensitive), the foreign
// list separator is replaced with the native one before applying to the child.
// Other keys are left untouched.
#[cfg(unix)]
#[test]
fn row_09_path_translates_semicolons_to_colons_on_unix() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("NODE_PATH=lib;extra;more")
        .arg(print_env_bin())
        .arg("NODE_PATH")
        .assert()
        .success()
        .stdout("NODE_PATH=lib:extra:more\n");
}

#[cfg(unix)]
#[test]
fn row_09_non_path_key_keeps_separator_on_unix() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_09_OTHER=a;b;c")
        .arg(print_env_bin())
        .arg("ROW_09_OTHER")
        .assert()
        .success()
        .stdout("ROW_09_OTHER=a;b;c\n");
}

// ---- Matrix row 8: `%VAR%` (Windows-style) auto-translate ----
// `%VAR%` is expanded from the parent process env on any platform — same
// `package.json` script works on both Windows shells and Unix shells.
#[test]
fn row_08_percent_var_percent_expands_from_parent() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env("ROW_08_PARENT", "winval")
        .arg("ROW_08_OUT=%ROW_08_PARENT%/tail")
        .arg(print_env_bin())
        .arg("ROW_08_OUT")
        .assert()
        .success()
        .stdout("ROW_08_OUT=winval/tail\n");
}

#[test]
fn row_08_unset_percent_var_expands_to_empty() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env_remove("ROW_08_MISSING")
        .arg("ROW_08_OUT=a%ROW_08_MISSING%b")
        .arg(print_env_bin())
        .arg("ROW_08_OUT")
        .assert()
        .success()
        .stdout("ROW_08_OUT=ab\n");
}

// ---- Matrix row 7: `$VAR` / `${VAR}` substitution ----
// Values like `$PARENT_VAR` and `${PARENT_VAR}` are expanded from the parent
// process env before being applied to the child. Unset vars expand to empty.
#[test]
fn row_07_unbraced_dollar_var_expands_from_parent() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env("ROW_07_PARENT", "expanded")
        .arg("ROW_07_OUT=$ROW_07_PARENT/tail")
        .arg(print_env_bin())
        .arg("ROW_07_OUT")
        .assert()
        .success()
        .stdout("ROW_07_OUT=expanded/tail\n");
}

#[test]
fn row_07_braced_dollar_var_expands_from_parent() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env("ROW_07_BR", "abc")
        .arg("ROW_07_OUT=${ROW_07_BR}xyz")
        .arg(print_env_bin())
        .arg("ROW_07_OUT")
        .assert()
        .success()
        .stdout("ROW_07_OUT=abcxyz\n");
}

#[test]
fn row_07_unset_var_expands_to_empty() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env_remove("ROW_07_MISSING")
        .arg("ROW_07_OUT=a${ROW_07_MISSING}b")
        .arg(print_env_bin())
        .arg("ROW_07_OUT")
        .assert()
        .success()
        .stdout("ROW_07_OUT=ab\n");
}

// ---- Matrix row 6: `--version` / `--help` flags ----
// Both bins recognise --help/-h and --version/-V only as the FIRST positional
// arg, exit 0, and print to stdout. Empty args print help to stderr and exit 1.
#[test]
fn row_06_help_long_flag_prints_usage_to_stdout() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage: cross-env "));
}

#[test]
fn row_06_help_short_flag_matches_long() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("-h")
        .assert()
        .success()
        .stdout(contains("Usage: cross-env "));
}

#[test]
fn row_06_version_long_flag_prints_pkg_version() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("{}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn row_06_version_short_flag_matches_long() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("-V")
        .assert()
        .success()
        .stdout(format!("{}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn row_06_shell_help_mentions_shell_binary() {
    Command::cargo_bin("cross-env-shell")
        .expect("cross-env-shell binary present")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("Usage: cross-env-shell "));
}

#[test]
fn row_06_shell_version_matches_pkg_version() {
    Command::cargo_bin("cross-env-shell")
        .expect("cross-env-shell binary present")
        .arg("--version")
        .assert()
        .success()
        .stdout(format!("{}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn row_06_no_args_prints_help_to_stderr_and_exits_nonzero() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .assert()
        .code(1)
        .stderr(contains("Usage: cross-env "));
}

#[test]
fn row_06_help_only_recognised_as_first_arg() {
    // `FOO=bar --help` — `--help` is not the first arg, so dispatch does not
    // short-circuit. parse() takes FOO=bar as env, `--help` as command;
    // which::which("--help") fails → exit 127 with an error on stderr.
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("FOO=bar")
        .arg("--help")
        .assert()
        .code(127)
        .stderr(contains("cross-env:"));
}

// ---- Matrix row 5: `--` argument terminator ----
// After `--`, env-parsing stops: subsequent KEY=VAL-shaped tokens reach the
// child as literal args, not as environment variables.
#[test]
fn row_05_double_dash_stops_env_parsing() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_05=applied")
        .arg("--")
        .arg(print_env_bin())
        .arg("ROW_05_LOOKS_LIKE=kv_but_isnt")
        .assert()
        .success()
        // print-env queries the literal key "ROW_05_LOOKS_LIKE=kv_but_isnt"
        // (no such env var) and prints "<unset>".
        .stdout("ROW_05_LOOKS_LIKE=kv_but_isnt=<unset>\n");
}

#[test]
fn row_05_double_dash_applied_envs_still_reach_child() {
    // Sanity: env vars set BEFORE `--` are still applied to the child.
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_05_VAR=hello")
        .arg("--")
        .arg(print_env_bin())
        .arg("ROW_05_VAR")
        .assert()
        .success()
        .stdout("ROW_05_VAR=hello\n");
}

// ---- Matrix row 12: SIGINT/SIGTERM forwarding to child ----
// cross-env, like upstream Node-cross-env, installs no explicit signal handler.
// A terminal signal (Ctrl+C) is delivered to the whole process group, so both
// cross-env and the child receive it simultaneously — both die. wait() from
// outside observes cross-env as signal-killed (status.code()==None,
// status.signal()==SIGTERM). The test kills the whole pgroup, asserts cross-env
// was actually signal-killed, and that wait() did not block waiting for the 30s
// sleep (which would imply the child outlived the wrapper).
#[cfg(unix)]
#[test]
fn row_12_pgroup_signal_kills_child_unix() {
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    use std::process::Stdio;
    use std::thread;
    use std::time::{Duration, Instant};

    let cross_env =
        std::env::var_os("CARGO_BIN_EXE_cross-env").expect("CARGO_BIN_EXE_cross-env set by cargo");

    let mut cmd = std::process::Command::new(cross_env);
    cmd.arg("ROW_12=x")
        .arg("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Isolate cross-env (and its sleep child) in a fresh pgroup so our SIGTERM
    // does not leak into the test runner.
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }
    let mut child = cmd.spawn().expect("spawn cross-env");

    let pgid = child.id() as i32;

    // Give cross-env time to fork sleep into our pgroup.
    thread::sleep(Duration::from_millis(300));

    unsafe {
        let rc = libc::killpg(pgid, libc::SIGTERM);
        assert_eq!(rc, 0, "killpg returned {rc}");
    }

    let started = Instant::now();
    let status = child.wait().expect("wait cross-env");
    let elapsed = started.elapsed();

    assert_eq!(
        status.signal(),
        Some(libc::SIGTERM),
        "cross-env should be killed by SIGTERM"
    );
    // If sleep had survived the signal, wait() would block for up to 30s.
    assert!(
        elapsed < Duration::from_secs(2),
        "wait took too long after pgroup signal: {elapsed:?}"
    );
}

// ---- Matrix row 13: Stdio inheritance ----
// Stdin from the parent must reach the child through cross-env.
// Stdout/stderr inheritance is implicitly verified by every `.stdout(...)` assertion.
#[test]
fn row_13_stdin_inherits_through_cross_env() {
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .arg("ROW_13=x")
        .arg(print_env_bin())
        .arg("--echo-stdin")
        .write_stdin("piped through\n")
        .assert()
        .success()
        .stdout("piped through\n");
}

// ---- Matrix row 14: Parent env passthrough + per-call override ----
#[test]
fn row_14_parent_env_passes_through() {
    // Var is set in parent env, not on the cross-env CLI — the child must inherit it.
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env("ROW_14_PARENT", "from_parent")
        .arg("ROW_14_OWN=ignored")
        .arg(print_env_bin())
        .arg("ROW_14_PARENT")
        .assert()
        .success()
        .stdout("ROW_14_PARENT=from_parent\n");
}

#[test]
fn row_14_cli_value_overrides_parent_value() {
    // Same name set in parent env and on the CLI — CLI value wins.
    Command::cargo_bin("cross-env")
        .expect("cross-env binary present")
        .env("ROW_14_OVERRIDE", "from_parent")
        .arg("ROW_14_OVERRIDE=from_cli")
        .arg(print_env_bin())
        .arg("ROW_14_OVERRIDE")
        .assert()
        .success()
        .stdout("ROW_14_OVERRIDE=from_cli\n");
}
