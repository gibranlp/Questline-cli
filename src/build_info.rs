pub fn build_label() -> String {
    format!(
        "v{}+{}{}",
        env!("CARGO_PKG_VERSION"),
        env!("QUESTLINE_BUILD_HASH"),
        env!("QUESTLINE_BUILD_DIRTY")
    )
}
