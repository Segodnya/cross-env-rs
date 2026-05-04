use std::ffi::{OsStr, OsString};
use std::process::ExitCode;

use anyhow::{anyhow, Result};

pub mod execute;
pub mod resolve;

pub use execute::{execute, DirectExecutor, Executor, ExitInfo, ShellExecutor};
pub use resolve::{resolve, EnvSource, Resolved, SystemEnv};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP_MAIN: &str = "Usage: cross-env [KEY=VALUE]... <command> [args]...

Set environment variables and run a command. Drop-in port of npm cross-env.

Options:
  -h, --help     Show this help and exit
  -V, --version  Show version and exit
";

const HELP_SHELL: &str = "Usage: cross-env-shell [KEY=VALUE]... <shell command>

Set environment variables and run a shell command (sh -c on Unix, cmd /c on Windows).
Useful when the command contains pipes, redirects, or shell-specific syntax.

Options:
  -h, --help     Show this help and exit
  -V, --version  Show version and exit
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Direct,
    Shell,
}

impl Mode {
    fn program_name(self) -> &'static str {
        match self {
            Mode::Direct => "cross-env",
            Mode::Shell => "cross-env-shell",
        }
    }

    fn help_text(self) -> &'static str {
        match self {
            Mode::Direct => HELP_MAIN,
            Mode::Shell => HELP_SHELL,
        }
    }
}

pub fn dispatch(args: Vec<OsString>, mode: Mode) -> ExitCode {
    if let Some(first) = args.first() {
        if first == "--help" || first == "-h" {
            print!("{}", mode.help_text());
            return ExitCode::SUCCESS;
        }
        if first == "--version" || first == "-V" {
            println!("{VERSION}");
            return ExitCode::SUCCESS;
        }
    } else {
        eprint!("{}", mode.help_text());
        return ExitCode::from(1);
    }

    let result = parse(args)
        .map(|p| resolve(p, &SystemEnv))
        .and_then(|r| run_with_mode(r, mode));

    match result {
        Ok(info) => ExitCode::from(clamp_code(info.to_exit_code())),
        Err(err) => {
            eprintln!("{}: {err:#}", mode.program_name());
            ExitCode::from(127)
        }
    }
}

fn run_with_mode(resolved: Resolved, mode: Mode) -> Result<ExitInfo> {
    match mode {
        Mode::Direct => execute(resolved, &DirectExecutor),
        Mode::Shell => execute(resolved, &ShellExecutor),
    }
}

fn clamp_code(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}

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
