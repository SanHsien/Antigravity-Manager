// Execute the production module's own regression tests without linking the
// desktop application's unrelated native runtime dependencies into this binary.
#[path = "../src/utils/crypto.rs"]
mod crypto;
