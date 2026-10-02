fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/generated/shell.ts");
    let generated = mstrchf_shell::features::typescript();
    if std::env::args().skip(1).any(|arg| arg == "--check") {
        assert_eq!(
            std::fs::read_to_string(path)
                .expect("generated shell DTO missing")
                .replace("\r\n", "\n"),
            generated,
            "shell DTO differs; run shell-types"
        );
    } else {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, generated).unwrap();
    }
}
