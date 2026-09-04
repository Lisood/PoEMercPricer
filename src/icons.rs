//! Icon art embedded in the exe by `build.rs`, so a downloaded copy of the
//! release (which is only the exe) still has skill and support icons.

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The bytes of `assets/icons/<kind>/<file>`, or `None` when the build had no
/// such file.
pub fn bytes(kind: &str, file: &str) -> Option<&'static [u8]> {
    let slice: &[(&str, &[u8])] = match kind {
        "skills" => &SKILLS,
        "supports" => &SUPPORTS,
        _ => return None,
    };
    slice
        .binary_search_by_key(&file, |(name, _)| name)
        .ok()
        .map(|i| slice[i].1)
}
