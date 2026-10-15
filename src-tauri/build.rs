const COMMON_CONTROLS_V6_MANIFEST_DEPENDENCY: &str = r#"/MANIFESTDEPENDENCY:"type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'""#;

fn main() {
    tauri_build::build();

    // Use Cargo's global link-arg directive so the library unit-test executable,
    // not only application binaries or integration tests, receives the dependency.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg={COMMON_CONTROLS_V6_MANIFEST_DEPENDENCY}");
    }
}
