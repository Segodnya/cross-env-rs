use std::process::ExitCode;

use cross_env_rs::{dispatch, Mode};

fn main() -> ExitCode {
    dispatch(std::env::args_os().skip(1).collect(), Mode::Direct)
}
