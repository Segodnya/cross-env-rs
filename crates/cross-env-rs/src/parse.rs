// CLI parsing phase: turns raw OS args into a `Parsed` form.
// Recognises `KEY=VAL` pairs (with `KEY` matching the env-var-name shape),
// the `--` terminator, and treats the first non-KV token as the command.

use std::ffi::{OsStr, OsString};

use anyhow::{anyhow, Result};

/// Tokens classified into env pairs, the command, and its remaining args.
pub struct Parsed {
    pub envs: Vec<(OsString, OsString)>,
    pub command: OsString,
    pub args: Vec<OsString>,
}

pub fn parse(input: Vec<OsString>) -> Result<Parsed> {
    let mut envs: Vec<(OsString, OsString)> = Vec::new();
    let mut iter = input.into_iter();
    let mut command: Option<OsString> = None;

    for arg in iter.by_ref() {
        if arg == "--" {
            command = iter.next();
            break;
        }
        if let Some((key, value)) = split_kv(&arg) {
            envs.push((key, value));
            continue;
        }
        command = Some(arg);
        break;
    }

    let command = command.ok_or_else(|| anyhow!("no command provided"))?;
    let args: Vec<OsString> = iter.collect();
    Ok(Parsed {
        envs,
        command,
        args,
    })
}

fn split_kv(arg: &OsStr) -> Option<(OsString, OsString)> {
    let s = arg.to_str()?;
    let eq = s.find('=')?;
    let key = &s[..eq];
    if !is_valid_env_name(key) {
        return None;
    }
    let value = &s[eq + 1..];
    Some((OsString::from(key), OsString::from(value)))
}

/// Single source of truth for the env-var-name shape: `[A-Za-z_][A-Za-z0-9_]*`.
/// Used both at parse time (to classify `KEY=VAL`) and at expansion time
/// (to validate `${NAME}` / `%NAME%`).
pub(crate) fn is_valid_env_name(s: &str) -> bool {
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

    fn os(s: &str) -> OsString {
        OsString::from(s)
    }

    #[test]
    fn parses_single_kv_and_command() {
        let parsed = parse(vec![os("FOO=bar"), os("echo"), os("hi")]).unwrap();
        assert_eq!(parsed.envs, vec![(os("FOO"), os("bar"))]);
        assert_eq!(parsed.command, os("echo"));
        assert_eq!(parsed.args, vec![os("hi")]);
    }

    #[test]
    fn parses_multiple_kv() {
        let parsed = parse(vec![
            os("FOO=1"),
            os("BAR=two"),
            os("BAZ="),
            os("node"),
            os("-e"),
            os("0"),
        ])
        .unwrap();
        assert_eq!(parsed.envs.len(), 3);
        assert_eq!(parsed.envs[2], (os("BAZ"), os("")));
        assert_eq!(parsed.command, os("node"));
    }

    #[test]
    fn first_non_kv_becomes_command() {
        let parsed = parse(vec![os("FOO=1"), os("./run.sh"), os("FOO=2")]).unwrap();
        assert_eq!(parsed.command, os("./run.sh"));
        assert_eq!(parsed.args, vec![os("FOO=2")]);
    }

    #[test]
    fn rejects_invalid_key() {
        let parsed = parse(vec![os("=cmd"), os("rest")]).unwrap();
        assert_eq!(parsed.envs, vec![]);
        assert_eq!(parsed.command, os("=cmd"));
    }

    #[test]
    fn rejects_key_starting_with_digit() {
        let parsed = parse(vec![os("1FOO=bar"), os("cmd")]).unwrap();
        assert!(parsed.envs.is_empty());
        assert_eq!(parsed.command, os("1FOO=bar"));
    }

    #[test]
    fn missing_command_errors() {
        assert!(parse(vec![os("FOO=bar")]).is_err());
        assert!(parse(vec![]).is_err());
    }

    #[test]
    fn double_dash_terminates_env_parsing() {
        let parsed = parse(vec![os("FOO=bar"), os("--"), os("BAZ=literal"), os("arg1")]).unwrap();
        assert_eq!(parsed.envs, vec![(os("FOO"), os("bar"))]);
        assert_eq!(parsed.command, os("BAZ=literal"));
        assert_eq!(parsed.args, vec![os("arg1")]);
    }

    #[test]
    fn double_dash_with_no_command_errors() {
        assert!(parse(vec![os("FOO=bar"), os("--")]).is_err());
        assert!(parse(vec![os("--")]).is_err());
    }

    #[test]
    fn env_name_validation() {
        assert!(is_valid_env_name("FOO"));
        assert!(is_valid_env_name("_FOO"));
        assert!(is_valid_env_name("FOO_BAR_42"));
        assert!(!is_valid_env_name(""));
        assert!(!is_valid_env_name("1FOO"));
        assert!(!is_valid_env_name("FOO-BAR"));
        assert!(!is_valid_env_name("FOO BAR"));
    }
}
