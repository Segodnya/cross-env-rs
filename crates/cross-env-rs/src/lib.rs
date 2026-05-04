use std::ffi::{OsStr, OsString};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const HELP_MAIN: &str = "Usage: cross-env [KEY=VALUE]... <command> [args]...

Set environment variables and run a command. Drop-in port of npm cross-env.

Options:
  -h, --help     Show this help and exit
  -V, --version  Show version and exit
";

pub const HELP_SHELL: &str = "Usage: cross-env-shell [KEY=VALUE]... <shell command>

Set environment variables and run a shell command (sh -c on Unix, cmd /c on Windows).
Useful when the command contains pipes, redirects, or shell-specific syntax.

Options:
  -h, --help     Show this help and exit
  -V, --version  Show version and exit
";

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
    if !is_valid_key(key) {
        return None;
    }
    let value = &s[eq + 1..];
    Some((OsString::from(key), OsString::from(value)))
}

fn is_valid_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn run(parsed: Parsed) -> Result<i32> {
    let resolved = which::which(&parsed.command)
        .with_context(|| format!("command not found: {}", parsed.command.to_string_lossy()))?;

    let mut cmd = Command::new(resolved);
    cmd.args(&parsed.args);
    for (k, v) in &parsed.envs {
        cmd.env(k, v);
    }

    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn: {}", parsed.command.to_string_lossy()))?;

    Ok(status.code().unwrap_or_else(|| signal_exit_code(&status)))
}

pub fn run_shell(parsed: Parsed) -> Result<i32> {
    let mut joined = String::new();
    push_arg(&mut joined, &parsed.command)?;
    for arg in &parsed.args {
        joined.push(' ');
        push_arg(&mut joined, arg)?;
    }

    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/d", "/s", "/c"]);
        c.arg(&joined);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c");
        c.arg(&joined);
        c
    };

    for (k, v) in &parsed.envs {
        cmd.env(k, v);
    }

    let status = cmd.status().context("failed to spawn shell")?;

    Ok(status.code().unwrap_or_else(|| signal_exit_code(&status)))
}

fn push_arg(buf: &mut String, arg: &OsStr) -> Result<()> {
    let s = arg
        .to_str()
        .ok_or_else(|| anyhow!("non-UTF-8 argument cannot be passed through to shell"))?;
    buf.push_str(s);
    Ok(())
}

#[cfg(unix)]
fn signal_exit_code(status: &std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status.signal().map(|s| 128 + s).unwrap_or(1)
}

#[cfg(not(unix))]
fn signal_exit_code(_status: &std::process::ExitStatus) -> i32 {
    1
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
    fn key_validation() {
        assert!(is_valid_key("FOO"));
        assert!(is_valid_key("_FOO"));
        assert!(is_valid_key("FOO_BAR_42"));
        assert!(!is_valid_key(""));
        assert!(!is_valid_key("1FOO"));
        assert!(!is_valid_key("FOO-BAR"));
        assert!(!is_valid_key("FOO BAR"));
    }
}
