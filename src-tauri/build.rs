fn main() {
    tauri_build::build();
    // Tauri links its Windows manifest into app binaries by default. Unit tests
    // using the mock runtime also need it to resolve Common Controls v6 APIs.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!(
            "cargo:rustc-link-search=native={}",
            std::env::var("OUT_DIR").unwrap()
        );
    }
}
