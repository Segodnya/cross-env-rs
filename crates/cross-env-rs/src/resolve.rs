// Resolution phase: turns raw `Parsed` tokens into a `Resolved` form ready
// for execution. Two responsibilities:
//   1. expand variable references in env values (via `crate::expand`),
//   2. translate PATH-list separators for whitelisted keys after expansion.

use std::ffi::{OsStr, OsString};

use crate::expand::{expand, EnvSource};
use crate::Parsed;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Local test fake. Mirrors `expand::tests::MapEnv` — kept separate so the
    // resolve-layer tests don't reach into the expand module's private test
    // utilities.
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
