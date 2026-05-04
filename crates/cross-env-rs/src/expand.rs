// Variable substitution layer.
// `expand()` performs `$VAR` / `${VAR}` / `%VAR%` substitution from the
// supplied `EnvSource`, unconditionally on any platform — that is part of
// the cross-env "drop-in" guarantee.

use std::ffi::{OsStr, OsString};

use crate::parse::is_valid_env_name;

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

// Variable substitution from `env`. Unset vars expand to empty
// (matches upstream `cross-env`). Single-pass — substituted text is not re-expanded.
// Non-UTF8 values are returned unchanged (no expansion possible).
//
// Recognised forms (any platform — translation is unconditional):
//   `$VAR`     — Unix-style, name is `[A-Za-z_][A-Za-z0-9_]*`
//   `${VAR}`   — Unix braced form
//   `%VAR%`    — Windows-style, name same shape as above
pub(crate) fn expand(value: &OsStr, env: &dyn EnvSource) -> OsString {
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
                    if is_valid_env_name(name) {
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
                if is_valid_env_name(name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    pub(crate) struct MapEnv(HashMap<OsString, OsString>);
    impl MapEnv {
        pub(crate) fn from_pairs(pairs: &[(&str, &str)]) -> Self {
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
}
