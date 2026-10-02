fn main() {
    // Type generation is a compile-only tool and needs no native runtime.
    if std::env::var_os("CARGO_FEATURE_NATIVE_SHELL").is_none() {
        return;
    }
    // Tauri's application resource is linked into binaries, not the library unit
    // test harness. Its Common Controls v6 imports still need an activation manifest.
    if std::env::var_os("CARGO_FEATURE_NATIVE_TESTS").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
    // Prevent a release build from embedding dev assets/configuration even when an
    // external caller supplies the dev merge config or development Cargo feature.
    if !cfg!(debug_assertions) {
        assert!(
            std::env::var_os("CARGO_FEATURE_SYNTHETIC_DEVELOPMENT").is_none(),
            "synthetic-development cannot be built for release"
        );
        assert!(
            std::env::var_os("TAURI_CONFIG").is_none(),
            "release config overrides are disabled at U1"
        );
    }
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(
                tauri_build::WindowsAttributes::new().window_icon_path("icons/icon.ico"),
            )
            .app_manifest(tauri_build::AppManifest::new().commands(&[
                "planner_snapshot",
                "planner_dispatch",
                "planner_features",
                "planner_replay_receipt",
            ])),
    )
    .expect("build shell manifest");
}
