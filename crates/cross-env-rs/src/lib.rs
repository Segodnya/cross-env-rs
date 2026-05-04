use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::Result;

pub mod execute;
pub mod expand;
pub mod parse;
pub mod resolve;

pub use execute::{execute, DirectExecutor, Executor, ExitInfo, ShellExecutor};
pub use expand::{EnvSource, SystemEnv};
pub use parse::{parse, Parsed};
pub use resolve::{resolve, Resolved};

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
        Ok(info) => ExitCode::from(info.to_process_exit_code()),
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
