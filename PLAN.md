# cross-env-rs — Drop-in parity roadmap

Granular ticket list for closing behavioural gaps with upstream
[`kentcdodds/cross-env`](https://github.com/kentcdodds/cross-env) (v8, Node ≥20).

Conventions used below:
- **Files** — paths to be touched. New files marked `(new)`.
- **AC** — acceptance criteria; every bullet must hold for the ticket to merge.
- **Depends** — tickets that must land first.

---

## Phase A — Drop-in parity (P0, blocking)

### A1 — Empty argv exits 0 silently

**Why:** Upstream exits 0 when invoked with no arguments; we print help to
stderr and exit 1.

**Files:** `crates/cross-env-rs/src/lib.rs`

**Depends:** —

**AC:**
- `dispatch(vec![], Mode::Direct)` returns `ExitCode::SUCCESS`, writes nothing
  to stderr/stdout.
- `dispatch(vec![], Mode::Shell)` — same.
- `--help` / `-h` / `--version` / `-V` behaviour unchanged.
- New integration test in `crates/cross-env-rs/tests/integration.rs`:
  invoking the binary with no args produces empty stdout, empty stderr,
  exit code 0.

---

### A2 — Strip outer matching quotes from RHS env value

**Why:** Upstream regex captures three precedence groups (`'…'` > `"…"` >
unquoted); a single outer pair is stripped. Without this, `FOO='bar'` ends up
as `FOO=\'bar\'` when argv is pre-joined (e.g. some Windows shells, or
`yarn run` on Windows).

**Files:** `crates/cross-env-rs/src/parse.rs`

**Depends:** —

**AC:**
- `parse(["FOO='bar'", "cmd"])` → env `("FOO", "bar")`.
- `parse(["FOO=\"bar\"", "cmd"])` → env `("FOO", "bar")`.
- `parse(["FOO='bar", "cmd"])` (unbalanced) → env `("FOO", "'bar")`.
- `parse(["FOO=''", "cmd"])` → env `("FOO", "")`.
- `parse(["FOO='{\"a\":1}'", "cmd"])` → env `("FOO", "{\"a\":1}")` — JSON
  payload survives.
- Quotes inside a different quote class are preserved verbatim:
  `FOO="b'ar"` → `b'ar`.
- Unit tests for each case in `parse::tests`.

---

### A3 — Support `${VAR:-default}` in `expand`

**Why:** Upstream supports POSIX-default syntax in env-value RHS expansion and
in Windows command-side rewrite (Phase A4). Currently unsupported.

**Files:** `crates/cross-env-rs/src/expand.rs`

**Depends:** —

**AC:**
- `${VAR:-fallback}` with `VAR` set and non-empty → value of `VAR`.
- `${VAR:-fallback}` with `VAR` unset → `fallback` literal.
- `${VAR:-fallback}` with `VAR` set to empty string → `fallback` (matches
  upstream `||`-style coercion).
- Default literal supports any character except `}` (no nested expansion —
  consistent with existing single-pass guarantee).
- `${:-x}` (empty name) → kept literal, like `${1bad}`.
- `${VAR:-}` (explicit empty default) → empty when `VAR` unset.
- New unit tests cover all five cases above.
- Existing tests in `expand::tests` keep passing untouched.

---

### A4 — Windows command-side `$VAR` → `%VAR%` rewrite

**Why:** On Windows, upstream walks the command and each arg, converting
`$VAR`/`${VAR}` to `%VAR%`, applying `${VAR:-default}`, and **dropping the
token entirely** if the variable is undefined and no default is given (fix for
upstream issue #145). Without this, `cross-env-rs FOO=bar node -e "process.env.FOO"`
through an npm script on Windows can leave `$FOO` as a literal.

**Files:**
- `crates/cross-env-rs/src/command_convert.rs` (new)
- `crates/cross-env-rs/src/resolve.rs` — call new module under `#[cfg(windows)]`
- `crates/cross-env-rs/src/lib.rs` — module declaration only

**Depends:** A3

**AC:**
- New module exposes `pub(crate) fn convert(value: &OsStr, env: &dyn EnvSource) -> OsString`.
- On non-Windows targets the function is a pass-through (returns input unchanged
  in O(1) — no allocation when no rewrite needed).
- On Windows: `$FOO` and `${FOO}` are replaced by their value when set; if
  unset and no default, the entire token is removed.
- `${FOO:-bar}` with `FOO` unset → `bar`.
- `EnvSource` passed in must include the env-vars set by the current invocation
  (overlay over `SystemEnv`), so `cross-env-rs FOO=bar echo $FOO` works on Windows.
  Implement an `OverlayEnv` adapter in `resolve.rs` (private), or extend
  `Resolved` to carry a merged view.
- `resolve()` invokes `convert` on `command` and every `args[i]` after
  expansion of env values, only on Windows.
- Unit tests in `command_convert.rs` (cfg-gated where needed): all four
  scenarios (set, unset+default, unset+no-default, braced).
- Unix path verified by existing tests staying green.

---

### A5 — Re-pin `APPDATA` on Windows child env

**Why:** Upstream explicitly re-sets `APPDATA` after merging user env, working
around a Windows spawn quirk where it gets lost.

**Files:** `crates/cross-env-rs/src/execute.rs`

**Depends:** —

**AC:**
- Under `#[cfg(windows)]`, after the `for (k, v) in &resolved.envs { cmd.env(k, v); }`
  loop, the child's `APPDATA` is set to the parent's `APPDATA` **only if the user
  did not explicitly override it** in `resolved.envs`.
- If user did override `APPDATA=foo`, that wins — pin does not clobber.
- A unit test that constructs a `Resolved` with and without an explicit
  `APPDATA` setter and asserts the resulting `Command` env (use
  `Command::get_envs()` introspection).

---

### A6 — Forward SIGTERM / SIGINT / SIGHUP to child (Unix)

**Why:** Upstream proxies signals so `kill <pid>` of the wrapper actually
terminates the wrapped process. Today we rely on POSIX process-group
inheritance, which works for Ctrl+C from a TTY but not for direct `kill`.

**Files:**
- `crates/cross-env-rs/src/execute.rs`
- `crates/cross-env-rs/Cargo.toml` — add `signal-hook = "0.3"` under
  `[target.'cfg(unix)'.dependencies]`; promote `libc` from dev-deps.

**Depends:** —

**AC:**
- Replace blocking `cmd.status()` with `cmd.spawn()` + `child.wait()` under
  `#[cfg(unix)]`.
- Register iterator for `SIGTERM`, `SIGINT`, `SIGHUP` via `signal-hook`; on
  receipt, send the same signal to `child.id()` via `libc::kill`.
- After `child.wait()` returns, deregister handlers (use `signal_hook::iterator::Signals`
  in a separate thread that exits when the child exits).
- `ExitInfo::from_status` semantics unchanged (`Signal(s)` → `128 + s`).
- New integration test (Unix-only): spawn a long-running child via the binary
  (`cross-env FOO=1 sleep 30`), send `SIGTERM` to the cross-env PID, assert
  child PID no longer exists within 2 s and the wrapper exit code is 143.
- Windows code path unchanged in this ticket.

---

### A7 — JS shim: switch `spawnSync` → `spawn` with signal forwarding

**Why:** `spawnSync` blocks the JS event loop, so signal handlers don't run
until the child exits. Long-running dev-servers don't shut down cleanly on
Ctrl+C.

**Files:**
- `npm/cross-env-rs/lib/run.js`
- `npm/cross-env-rs/test/shim.test.js`

**Depends:** —

**AC:**
- `run.js` uses async `spawn` (from `child_process`), inherits stdio.
- Signal handlers attached for `SIGTERM`, `SIGINT`, `SIGBREAK` (Windows),
  `SIGHUP`. On receipt, call `child.kill(sig)`.
- On `child.exit(code, signal)`:
  - if `code != null` → `process.exit(code)`.
  - if `signal` set → re-raise via `process.kill(process.pid, signal)` so
    callers see the actual signal (not exit 1).
- All existing shim tests still pass; new tests for:
  - Signal-forward path (mock `spawn` returns EventEmitter; emit a signal,
    assert `child.kill` called with that signal).
  - Re-raise path (emit `exit(null, 'SIGTERM')`; assert
    `sendSignal('SIGTERM')` invoked).
  - Normal exit (emit `exit(7, null)`; assert `exit(7)` called).
- Default export shape (`run`, `detectPlatformPackage`, `detectLinuxLibc`,
  `SUPPORTED`) unchanged.

---

## Phase B — Parity edge cases (P1)

### B1 — Backslash-escape rule in RHS expansion

**Why:** Upstream lets users escape `$` with `\`: odd count of `\` before `$`
keeps `$VAR` literal (consuming one `\`); even count halves the backslashes
and substitutes.

**Files:** `crates/cross-env-rs/src/expand.rs`

**Depends:** A3 (avoid touching expand twice)

**AC:**
- `\$FOO` → `$FOO` (literal, one `\` consumed).
- `\\$FOO` (two `\` then `$`) → `\` + value of FOO.
- `\\\$FOO` → `\\$FOO` (literal).
- Handles `${VAR}` form too: `\${VAR}` → `${VAR}` literal.
- New unit tests for each case; existing tests still pass.

---

### B2 — Backslash-unescape pass on post-setter argv

**Why:** Upstream applies `/\\\\|(\\)?'|([\\])(?=[$"\\])/g` to every argv
token after the env setters: `\\` → `\`, `\'` → `'`, single `\` before
`$ " \` is stripped. Mostly relevant for Windows users who pass escaped
arguments through layered shells.

**Files:** `crates/cross-env-rs/src/parse.rs`

**Depends:** —

**AC:**
- New `pub(crate) fn unescape_arg(s: &OsStr) -> OsString`.
- Applied to `command` and each element of `args` in `parse()`.
- Unit tests for: `\\` → `\`; `\'` → `'`; `\$` → `$`; `\"` → `"`; `\a`
  → `\a` (no-op for unrecognised escape).
- Behaviour matches upstream regex on a representative set of strings
  (document the chosen test vectors as a comment in tests).

---

### B3 — Escape-aware PATH/NODE_PATH separator translation

**Why:** Upstream lets users escape the separator with `\`: odd count of `\`
before `:` (Unix→Win) or `;` (Win→Unix) escapes it (one `\` consumed); even
count translates as normal. Current `replace` is blind.

**Files:** `crates/cross-env-rs/src/resolve.rs`

**Depends:** —

**AC:**
- `translate_path_separators` walks the string, counting `\` runs before
  each candidate separator.
- Unit tests:
  - `a\;b` on Unix → `a;b` (escape consumed, no translation).
  - `a\\;b` on Unix → `a\:b` (even count: one `\` left, translated).
  - Windows symmetric: `a\:b` → `a:b`; `a\\:b` → `a\;b`.
- Existing PATH-list translation tests stay green.
