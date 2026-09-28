use mstrchf_contracts::schema::{check_generated, write_generated};

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root");
    let result = match std::env::args().nth(1).as_deref() {
        None | Some("--write") => write_generated(root),
        Some("--check") => check_generated(root),
        Some(_) => {
            eprintln!("usage: generate-contracts [--write|--check]");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        let path = error
            .issues
            .0
            .first()
            .map(|issue| issue.path.as_str())
            .unwrap_or("");
        eprintln!("contract generation failed: {error} {path}");
        std::process::exit(1);
    }
}
