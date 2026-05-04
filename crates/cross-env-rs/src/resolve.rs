// Resolution phase: turns raw `Parsed` tokens into a `Resolved` form ready for execution.
// `expand()` performs `$VAR`/`${VAR}` substitution from the supplied `EnvSource`.
// PR 7 adds `%VAR%` translation; PR 8 adds PATH-list separator translation.

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
        .map(|(k, v)| (k, expand(&v, env)))
        .collect();
    Resolved {
        envs,
        command: parsed.command,
        args: parsed.args,
    }
}

// `$VAR` and `${VAR}` substitution from `env`. Unset vars expand to empty
// (matches upstream `cross-env`). Single-pass — substituted text is not re-expanded.
// Non-UTF8 values are returned unchanged (no expansion possible).
fn expand(value: &OsStr, env: &dyn EnvSource) -> OsString {
    let Some(s) = value.to_str() else {
        return value.to_owned();
    };
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if bytes[i] == b'$' && i + 1 < s.len() {
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
        }
        // Copy one UTF-8 char (multi-byte safe).
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    OsString::from(out)
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
}
