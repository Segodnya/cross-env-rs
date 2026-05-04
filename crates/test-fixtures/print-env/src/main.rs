// Test fixture binary used by `cross-env-rs` integration tests.
// Prints requested environment variables in `KEY=VALUE` form, one per line.
// With no arguments, prints all environment variables.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        for (k, v) in std::env::vars() {
            println!("{k}={v}");
        }
        return;
    }
    for k in &args {
        println!("{}={}", k, std::env::var(k).unwrap_or_default());
    }
}
