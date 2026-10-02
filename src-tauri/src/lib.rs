pub mod features;
pub mod receipt;

#[cfg(all(
    debug_assertions,
    any(
        feature = "synthetic-development",
        all(test, feature = "store-boundary")
    )
))]
mod boundary;

#[cfg(all(debug_assertions, feature = "synthetic-development"))]
mod development;

/// Production intentionally has neither a Store nor registered planner handlers.
#[cfg(feature = "native-shell")]
pub fn run() {
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    development::run();
    #[cfg(not(all(debug_assertions, feature = "synthetic-development")))]
    tauri::Builder::default()
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("run disabled production shell");
}

#[cfg(all(test, debug_assertions, feature = "native-tests"))]
mod tests {
    #[test]
    fn only_the_exact_development_window_and_local_origins_are_allowed() {
        assert!(
            super::development::authorise("development-main", "http://127.0.0.1:1420/").is_ok()
        );
        assert!(super::development::authorise("other", "http://127.0.0.1:1420/").is_err());
        for url in [
            "http://user:password@127.0.0.1:1420/",
            "http://127.0.0.1:1421/",
            "http://tauri.localhost:1420/",
            "tauri://localhost:1420/",
            "https://tauri.localhost/",
            "file:///index.html",
            "http://evil.example/",
        ] {
            assert!(
                super::development::authorise("development-main", url).is_err(),
                "{url}"
            );
        }
        for url in ["http://tauri.localhost/", "tauri://localhost/"] {
            assert!(super::development::authorise("development-main", url).is_ok());
        }

        assert!(super::development::authorise("development-main", "https://example.com/").is_err());
        assert!(
            super::development::authorise("development-main", "http://localhost:1420.evil/")
                .is_err()
        );
    }
}

#[cfg(all(test, debug_assertions, feature = "store-boundary"))]
#[path = "../tests/receipt_adapter.rs"]
mod receipt_adapter;
#[cfg(all(test, debug_assertions, feature = "store-boundary"))]
#[path = "../tests/receipt_fixture.rs"]
mod receipt_fixture;
