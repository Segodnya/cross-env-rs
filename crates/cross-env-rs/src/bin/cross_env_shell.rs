use std::ffi::OsString;
use std::process::ExitCode;

use cross_env_rs::{parse, run_shell, HELP_SHELL, VERSION};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if let Some(first) = args.first() {
        if first == "--help" || first == "-h" {
            print!("{HELP_SHELL}");
            return ExitCode::SUCCESS;
        }
        if first == "--version" || first == "-V" {
            println!("{VERSION}");
            return ExitCode::SUCCESS;
        }
    } else {
        eprint!("{HELP_SHELL}");
        return ExitCode::from(1);
    }

    let result = parse(args).and_then(run_shell);

    match result {
        Ok(code) => ExitCode::from(clamp_code(code)),
        Err(err) => {
            eprintln!("cross-env-shell: {err:#}");
            ExitCode::from(127)
        }
    }
}

fn clamp_code(code: i32) -> u8 {
    if (0..=255).contains(&code) {
        code as u8
    } else {
        1
    }
}
