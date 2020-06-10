/// Detect a docs.rs build environment. This probably breaks Crater, but libsigrok isn't installed
/// there anyways so...
fn is_docs_rs() -> bool {
    std::env::var_os("USER")
        .map(|var| var == "crates-build-env")
        .unwrap_or(false)
}

fn main() {
    if is_docs_rs() {
        return;
    }
    pkg_config::probe_library("libsigrok").unwrap();
}
