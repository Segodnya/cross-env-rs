// Resolution phase: turns raw `Parsed` tokens into a `Resolved` form ready for execution.
// Currently a structural pass-through; PR 6–8 fill in `expand()` for `$VAR`/`${VAR}`/`%VAR%`/PATH.

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

// Identity placeholder. PR 6 adds `$VAR`/`${VAR}`; PR 7 adds `%VAR%`; PR 8 adds PATH-list translate.
fn expand(value: &OsStr, _env: &dyn EnvSource) -> OsString {
    value.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEnv;
    impl EnvSource for FakeEnv {
        fn get(&self, _key: &OsStr) -> Option<OsString> {
            None
        }
    }

    #[test]
    fn resolve_currently_passes_through() {
        let parsed = Parsed {
            envs: vec![(OsString::from("FOO"), OsString::from("bar"))],
            command: OsString::from("cmd"),
            args: vec![OsString::from("arg")],
        };
        let resolved = resolve(parsed, &FakeEnv);
        assert_eq!(resolved.envs[0].0, OsString::from("FOO"));
        assert_eq!(resolved.envs[0].1, OsString::from("bar"));
        assert_eq!(resolved.command, OsString::from("cmd"));
    }
}
