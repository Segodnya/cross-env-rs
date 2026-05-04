// Resolution phase: turns raw `Parsed` tokens into a `Resolved` form ready for execution.
// `expand()` performs `$VAR`/`${VAR}`/`%VAR%` substitution from the supplied `EnvSource`,
// unconditionally on any platform — that is the cross-env "drop-in" guarantee.
// For values of PATH-list keys (PATH, NODE_PATH), the foreign list separator is
// translated to the native one after expansion.

use std::ffi::{OsStr, OsString};

use crate::Parsed;

/// Source of pre-existing env vars consulted during value expansion.
/// Production uses `SystemEnv`; tests can supply a fake.
pub trait EnvSource {
    fn get(&self, key: &OsStr) -> Option<OsString>;
}

/// Real process environment.
pub struct SystemEnv;

impl EnvSource for SystemEnv {
    fn get(&self, key: &OsStr) -> Option<OsString> {
        std::env::var_os(key)
    }
}

/// Post-expansion env + command + args.
#[derive(Debug)]
pub struct Resolved {
    pub envs: Vec<(OsString, OsString)>,
    pub command: OsString,
    pub args: Vec<OsString>,
}

pub fn resolve(parsed: Parsed, env: &dyn EnvSource) -> Resolved {
    let envs = parsed
        .envs
        .into_iter()
        .map(|(k, v)| {
            let expanded = expand(&v, env);
            let value = if is_path_list_key(&k) {
                translate_path_separators(expanded)
            } else {
                expanded
            };
            (k, value)
        })
        .collect();
    Resolved {
        envs,
        command: parsed.command,
        args: parsed.args,
    }
}

// Variable substitution from `env`. Unset vars expand to empty
// (matches upstream `cross-env`). Single-pass — substituted text is not re-expanded.
// Non-UTF8 values are returned unchanged (no expansion possible).
//
// Recognised forms (any platform — translation is unconditional):
//   `$VAR`     — Unix-style, name is `[A-Za-z_][A-Za-z0-9_]*`
//   `${VAR}`   — Unix braced form
//   `%VAR%`    — Windows-style, name same shape as above
fn expand(value: &OsStr, env: &dyn EnvSource) -> OsString {
    let Some(s) = value.to_str() else {
        return value.to_owned();
    };
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let b = bytes[i];
        if b == b'$' && i + 1 < s.len() {
            let next = bytes[i + 1];
            if next == b'{' {
                if let Some(rel_end) = s[i + 2..].find('}') {
                    let name = &s[i + 2..i + 2 + rel_end];
                    if is_valid_var_name(name) {
                        if let Some(v) = env.get(OsStr::new(name)) {
                            out.push_str(&v.to_string_lossy());
                        }
                        i += 2 + rel_end + 1;
                        continue;
                    }
                }
            } else if next == b'_' || next.is_ascii_alphabetic() {
                let start = i + 1;
                let mut end = start;
                while end < s.len() && (bytes[end] == b'_' || bytes[end].is_ascii_alphanumeric()) {
                    end += 1;
                }
                let name = &s[start..end];
                if let Some(v) = env.get(OsStr::new(name)) {
                    out.push_str(&v.to_string_lossy());
                }
                i = end;
                continue;
            }
        } else if b == b'%' && i + 1 < s.len() {
            if let Some(rel_end) = s[i + 1..].find('%') {
                let name = &s[i + 1..i + 1 + rel_end];
                if is_valid_var_name(name) {
                    if let Some(v) = env.get(OsStr::new(name)) {
                        out.push_str(&v.to_string_lossy());
                    }
                    i += 1 + rel_end + 1;
                    continue;
                }
            }
        }
        // Copy one UTF-8 char (multi-byte safe).
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    OsString::from(out)
}

// Matches upstream `cross-env`: only PATH and NODE_PATH get separator translation.
// Comparison is case-insensitive (upstream uses `key.toUpperCase()`).
const PATH_LIST_KEYS: &[&str] = &["PATH", "NODE_PATH"];

fn is_path_list_key(key: &OsStr) -> bool {
    let Some(s) = key.to_str() else {
        return false;
    };
    let upper = s.to_ascii_uppercase();
    PATH_LIST_KEYS.iter().any(|k| *k == upper)
}

// On Unix the native list separator is `:`; on Windows it is `;`. Translation
// goes one direction only — we replace the foreign separator with the native
// one. Caveat (matches upstream): on Windows the naive `:` → `;` replace will
// also rewrite drive-letter colons (`C:\foo:D:\bar` → `C;\foo;D;\bar`).
fn translate_path_separators(value: OsString) -> OsString {
    let Some(s) = value.to_str() else {
        return value;
    };
    #[cfg(unix)]
    {
        if !s.contains(';') {
            return value;
        }
        OsString::from(s.replace(';', ":"))
    }
    #[cfg(windows)]
    {
        if !s.contains(':') {
            return value;
        }
        OsString::from(s.replace(':', ";"))
    }
}

fn is_valid_var_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapEnv(HashMap<OsString, OsString>);
    impl MapEnv {
        fn from_pairs(pairs: &[(&str, &str)]) -> Self {
            Self(
                pairs
                    .iter()
                    .map(|(k, v)| (OsString::from(*k), OsString::from(*v)))
                    .collect(),
            )
        }
    }
    impl EnvSource for MapEnv {
        fn get(&self, key: &OsStr) -> Option<OsString> {
            self.0.get(key).cloned()
        }
    }

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn no_dollar_passes_through() {
        let env = MapEnv::from_pairs(&[("HOME", "/home/u")]);
        assert_eq!(expand(OsStr::new("plain text"), &env), os("plain text"));
        assert_eq!(expand(OsStr::new(""), &env), os(""));
    }

    #[test]
    fn expands_unbraced_dollar_var() {
        let env = MapEnv::from_pairs(&[("HOME", "/home/u")]);
        assert_eq!(expand(OsStr::new("$HOME/bin"), &env), os("/home/u/bin"));
    }

    #[test]
    fn expands_braced_dollar_var() {
        let env = MapEnv::from_pairs(&[("HOME", "/home/u")]);
        assert_eq!(expand(OsStr::new("${HOME}bar"), &env), os("/home/ubar"));
    }

    #[test]
    fn unset_var_expands_to_empty() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("a${MISSING}b"), &env), os("ab"));
        assert_eq!(expand(OsStr::new("a$MISSING/b"), &env), os("a/b"));
    }

    #[test]
    fn lone_dollar_kept_literal() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("price: $"), &env), os("price: $"));
        assert_eq!(expand(OsStr::new("$ "), &env), os("$ "));
        assert_eq!(expand(OsStr::new("$1abc"), &env), os("$1abc"));
    }

    #[test]
    fn unterminated_brace_kept_literal() {
        let env = MapEnv::from_pairs(&[("X", "ok")]);
        assert_eq!(expand(OsStr::new("${X"), &env), os("${X"));
    }

    #[test]
    fn invalid_braced_name_kept_literal() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("${1bad}"), &env), os("${1bad}"));
        assert_eq!(expand(OsStr::new("${}"), &env), os("${}"));
    }

    #[test]
    fn no_recursive_expansion() {
        // X → "$Y", Y → "deep". Single pass: $X becomes "$Y" literally.
        let env = MapEnv::from_pairs(&[("X", "$Y"), ("Y", "deep")]);
        assert_eq!(expand(OsStr::new("$X"), &env), os("$Y"));
    }

    #[test]
    fn multiple_substitutions_in_one_value() {
        let env = MapEnv::from_pairs(&[("A", "1"), ("B", "2")]);
        assert_eq!(expand(OsStr::new("$A-${B}-$A"), &env), os("1-2-1"));
    }

    #[test]
    fn expands_percent_var_percent() {
        let env = MapEnv::from_pairs(&[("CD", "C:\\proj")]);
        assert_eq!(expand(OsStr::new("%CD%/src"), &env), os("C:\\proj/src"));
    }

    #[test]
    fn unset_percent_var_expands_to_empty() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("a%MISSING%b"), &env), os("ab"));
    }

    #[test]
    fn lone_percent_kept_literal() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("100% done"), &env), os("100% done"));
    }

    #[test]
    fn invalid_percent_name_kept_literal() {
        let env = MapEnv::from_pairs(&[]);
        assert_eq!(expand(OsStr::new("%1bad%"), &env), os("%1bad%"));
        assert_eq!(expand(OsStr::new("%%"), &env), os("%%"));
    }

    #[test]
    fn dollar_and_percent_intermix() {
        let env = MapEnv::from_pairs(&[("A", "x"), ("B", "y")]);
        assert_eq!(expand(OsStr::new("$A-%B%"), &env), os("x-y"));
    }

    #[test]
    fn resolve_applies_expand_to_each_env_value() {
        let env = MapEnv::from_pairs(&[("HOME", "/h")]);
        let parsed = Parsed {
            envs: vec![(os("OUT"), os("$HOME/out"))],
            command: os("cmd"),
            args: vec![],
        };
        let resolved = resolve(parsed, &env);
        assert_eq!(resolved.envs[0].1, os("/h/out"));
    }

    #[test]
    fn path_list_key_recognition() {
        assert!(is_path_list_key(OsStr::new("PATH")));
        assert!(is_path_list_key(OsStr::new("path")));
        assert!(is_path_list_key(OsStr::new("Path")));
        assert!(is_path_list_key(OsStr::new("NODE_PATH")));
        assert!(is_path_list_key(OsStr::new("node_path")));
        assert!(!is_path_list_key(OsStr::new("HOME")));
        // Upstream's whitelist is narrow on purpose — these are NOT translated.
        assert!(!is_path_list_key(OsStr::new("MANPATH")));
        assert!(!is_path_list_key(OsStr::new("LD_LIBRARY_PATH")));
    }

    #[test]
    fn translate_returns_value_unchanged_when_no_foreign_sep() {
        let v = os("just/a/path");
        assert_eq!(translate_path_separators(v.clone()), v);
    }

    #[cfg(unix)]
    #[test]
    fn translate_replaces_semicolon_with_colon_on_unix() {
        assert_eq!(translate_path_separators(os("a;b;c")), os("a:b:c"));
        assert_eq!(translate_path_separators(os(";leading")), os(":leading"));
        assert_eq!(translate_path_separators(os("trailing;")), os("trailing:"));
    }

    #[cfg(windows)]
    #[test]
    fn translate_replaces_colon_with_semicolon_on_windows() {
        assert_eq!(translate_path_separators(os("a:b:c")), os("a;b;c"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_translates_path_list_keys_only() {
        let env = MapEnv::from_pairs(&[]);
        let parsed = Parsed {
            envs: vec![
                (os("PATH"), os("a;b;c")),         // translated
                (os("node_path"), os("x;y")),      // translated (case-insensitive)
                (os("HOME"), os("ignore;me")),     // not a path key — left as-is
                (os("MANPATH"), os("not;in;set")), // upstream whitelist excludes this
            ],
            command: os("cmd"),
            args: vec![],
        };
        let resolved = resolve(parsed, &env);
        assert_eq!(resolved.envs[0].1, os("a:b:c"));
        assert_eq!(resolved.envs[1].1, os("x:y"));
        assert_eq!(resolved.envs[2].1, os("ignore;me"));
        assert_eq!(resolved.envs[3].1, os("not;in;set"));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_expands_then_translates() {
        // Order matters: $VAR is substituted first, THEN the result has
        // separators translated. So `$EXTRA` resolves to `x;y`, and the whole
        // value becomes `head:x:y` on Unix.
        let env = MapEnv::from_pairs(&[("EXTRA", "x;y")]);
        let parsed = Parsed {
            envs: vec![(os("PATH"), os("head;$EXTRA"))],
            command: os("cmd"),
            args: vec![],
        };
        let resolved = resolve(parsed, &env);
        assert_eq!(resolved.envs[0].1, os("head:x:y"));
    }
}
