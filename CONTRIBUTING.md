# Contributing

PoEMercPricer is a small Rust overlay. Please keep changes scoped.

1. `cargo test` must pass.
2. Scoring formula changes need a unit test (see `tests/scoring.rs`).
3. Do not add process injection, packet sniffing, or input playback.
4. Windows is the supported overlay/OCR platform.
5. `cargo fmt` before committing.

Scoring math lives in `src/scoring/`. Overlay UX lives in `src/app.rs`.

## Reporting bugs

Use the bug report issue template. Crop screenshots to the mercenary
panel; do not include chat, friends list, or account names.

Security issues: see SECURITY.md.
