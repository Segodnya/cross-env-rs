// Test fixture binary used by `cross-env-rs` integration tests.
// Modes:
//   `print-env --echo-stdin`     — copy stdin to stdout (used by row 13 stdio test).
//   `print-env KEY [KEY...]`     — print `KEY=value` per line; `<unset>` for missing.
//   `print-env`                  — print all environment variables.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("--echo-stdin") {
        std::io::copy(&mut std::io::stdin(), &mut std::io::stdout()).expect("copy stdin to stdout");
        return;
    }

    if args.is_empty() {
        for (k, v) in std::env::vars() {
            println!("{k}={v}");
        }
        return;
    }
    for k in &args {
        match std::env::var_os(k) {
            Some(v) => println!("{}={}", k, v.to_string_lossy()),
            None => println!("{}=<unset>", k),
        }
    }
}
