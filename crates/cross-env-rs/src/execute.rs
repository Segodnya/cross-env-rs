// Execution phase: turns a `Resolved` into a spawned child and reports its outcome.
// Two adapters: `DirectExecutor` (which-resolved binary) and `ShellExecutor` (sh / cmd).
// Shared post-spawn logic — env application, status capture — lives in `execute()`.

use std::ffi::OsStr;
use std::process::{Command, ExitStatus};

use anyhow::{anyhow, Context, Result};

use crate::resolve::Resolved;

/// Outcome of running a child process.
/// Caller maps this to a process-level exit code via `to_exit_code`.
#[derive(Debug, PartialEq, Eq)]
pub enum ExitInfo {
    Code(i32),
    Signal(i32),
    Unknown,
}

impl ExitInfo {
    pub fn from_status(status: &ExitStatus) -> Self {
        if let Some(c) = status.code() {
            return ExitInfo::Code(c);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(s) = status.signal() {
                return ExitInfo::Signal(s);
            }
        }
        ExitInfo::Unknown
    }

    /// Map to the integer exit code we propagate to the OS.
    /// `Signal(s)` → `128 + s` (POSIX convention). `Unknown` → `1`.
    pub fn to_exit_code(&self) -> i32 {
        match self {
            ExitInfo::Code(c) => *c,
            ExitInfo::Signal(s) => 128 + *s,
            ExitInfo::Unknown => 1,
        }
    }
}

/// Spawn strategy. `prepare_command` builds the `Command`; the shared `execute()`
/// applies envs and captures status. Two concrete adapters: Direct, Shell.
pub trait Executor {
    fn prepare_command(&self, resolved: &Resolved) -> Result<Command>;
    fn fail_context(&self, resolved: &Resolved) -> String;
}

/// Spawns the command directly via a `which`-resolved absolute path.
pub struct DirectExecutor;

impl Executor for DirectExecutor {
    fn prepare_command(&self, resolved: &Resolved) -> Result<Command> {
        let path = which::which(&resolved.command).with_context(|| {
            format!("command not found: {}", resolved.command.to_string_lossy())
        })?;
        let mut cmd = Command::new(path);
        cmd.args(&resolved.args);
        Ok(cmd)
    }

    fn fail_context(&self, resolved: &Resolved) -> String {
        format!("failed to spawn: {}", resolved.command.to_string_lossy())
    }
}

/// Spawns the command via the platform shell (`sh -c` on Unix, `cmd /d /s /c` on Windows).
pub struct ShellExecutor;

impl Executor for ShellExecutor {
    fn prepare_command(&self, resolved: &Resolved) -> Result<Command> {
        let mut joined = String::new();
        push_arg(&mut joined, &resolved.command)?;
        for arg in &resolved.args {
            joined.push(' ');
            push_arg(&mut joined, arg)?;
        }
        Ok(shell_command(&joined))
    }

    fn fail_context(&self, _resolved: &Resolved) -> String {
        "failed to spawn shell".to_string()
    }
}

#[cfg(unix)]
fn shell_command(joined: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c");
    c.arg(joined);
    c
}

#[cfg(windows)]
fn shell_command(joined: &str) -> Command {
    let mut c = Command::new("cmd");
    c.args(["/d", "/s", "/c"]);
    c.arg(joined);
    c
}

fn push_arg(buf: &mut String, arg: &OsStr) -> Result<()> {
    let s = arg
        .to_str()
        .ok_or_else(|| anyhow!("non-UTF-8 argument cannot be passed through to shell"))?;
    buf.push_str(s);
    Ok(())
}

/// Run a `Resolved` with the given executor: apply envs, spawn, capture status.
pub fn execute(resolved: Resolved, executor: &dyn Executor) -> Result<ExitInfo> {
    let mut cmd = executor.prepare_command(&resolved)?;
    for (k, v) in &resolved.envs {
        cmd.env(k, v);
    }
    let status = cmd
        .status()
        .with_context(|| executor.fail_context(&resolved))?;
    Ok(ExitInfo::from_status(&status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_from_code() {
        assert_eq!(ExitInfo::Code(0).to_exit_code(), 0);
        assert_eq!(ExitInfo::Code(42).to_exit_code(), 42);
    }

    #[test]
    fn exit_code_from_signal() {
        assert_eq!(ExitInfo::Signal(9).to_exit_code(), 137);
        assert_eq!(ExitInfo::Signal(15).to_exit_code(), 143);
    }

    #[test]
    fn exit_code_from_unknown() {
        assert_eq!(ExitInfo::Unknown.to_exit_code(), 1);
    }
}
