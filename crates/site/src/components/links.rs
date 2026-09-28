//! The destinations the hero and the footer both link to, so the two rows
//! cannot disagree.

pub const CRATES_IO: &str = "https://crates.io/crates/playwright-rs";
pub const DOCS_RS: &str = "https://docs.rs/playwright-rs";
pub const GITHUB: &str = "https://github.com/padamson/playwright-rust";
/// Relative on purpose: the architecture tree ships inside each version
/// snapshot, so the link must resolve under /dev/ or /vX.Y.Z/, not the root.
pub const ARCHITECTURE: &str = "architecture/";
