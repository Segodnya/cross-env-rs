use std::ffi::OsString;
use std::process::ExitCode;

use cross_env_rs::{execute, parse, resolve, DirectExecutor, SystemEnv, HELP_MAIN, VERSION};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();

    if let Some(first) = args.first() {
        if first == "--help" || first == "-h" {
            print!("{HELP_MAIN}");
            return ExitCode::SUCCESS;
        }
        if first == "--version" || first == "-V" {
            println!("{VERSION}");
            return ExitCode::SUCCESS;
        }
    } else {
        eprint!("{HELP_MAIN}");
        return ExitCode::from(1);
    }

    let result = parse(args)
        .map(|p| resolve(p, &SystemEnv))
        .and_then(|r| execute(r, &DirectExecutor));

    match result {
        Ok(info) => ExitCode::from(clamp_code(info.to_exit_code())),
        Err(err) => {
            eprintln!("cross-env: {err:#}");
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
