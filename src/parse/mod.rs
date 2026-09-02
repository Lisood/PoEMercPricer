mod ocr_text;
mod warrant;

pub use ocr_text::parse_ocr_lines;
pub use warrant::{looks_like_warrant, parse_warrant_text};
