pub fn version_label() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}
