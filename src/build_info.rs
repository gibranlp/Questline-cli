pub const BUILD_NUMBER: u32 = 40;

pub fn build_label() -> String {
    format!("v{}.{:04}", env!("CARGO_PKG_VERSION"), BUILD_NUMBER)
}
