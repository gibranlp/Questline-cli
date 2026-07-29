pub const BUILD_NUMBER: u32 = 24;

pub fn build_label() -> String {
    format!("v{}.{:04}", env!("CARGO_PKG_VERSION"), BUILD_NUMBER)
}
