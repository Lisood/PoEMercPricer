use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/branding/app-icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/branding/app-icon.ico")
            .compile()
            .expect("compile the PoEMercPricer Windows icon resource");
    }

    // The embedded JSON is pretty-printed in the repo for review; strip the
    // whitespace outside string literals so the exe carries 164 KB instead of
    // 354 KB, and the same bytes regardless of the checkout's line endings.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    for name in [
        "catalog-3.29.json",
        "trade-stats-3.29.json",
        "warrant-prices-3.29.json",
    ] {
        println!("cargo:rerun-if-changed=assets/{name}");
        let src = std::fs::read_to_string(format!("assets/{name}"))
            .unwrap_or_else(|e| panic!("read assets/{name}: {e}"));
        std::fs::write(Path::new(&out_dir).join(name), minify_json(&src))
            .unwrap_or_else(|e| panic!("write minified {name}: {e}"));
    }

    // The release uploads only the exe, so the icon art has to travel inside
    // it: without this a downloaded copy has no support templates and reports
    // every support as unresolved.
    let mut icons = String::new();
    for (slice, dir) in [
        ("SKILLS", "assets/icons/skills"),
        ("SUPPORTS", "assets/icons/supports"),
    ] {
        println!("cargo:rerun-if-changed={dir}");
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read {dir}: {e}"))
            .map(|e| e.expect("dir entry").file_name().to_string_lossy().into())
            .filter(|name: &String| name.ends_with(".webp"))
            .collect();
        // First-seen order decides which art id owns a byte-identical
        // template, so keep it stable across machines.
        names.sort();
        icons.push_str(&format!(
            "pub static {slice}: [(&str, &[u8]); {}] = [\n",
            names.len()
        ));
        for name in names {
            let path = format!("{}/{dir}/{name}", manifest.replace('\\', "/"));
            icons.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
        }
        icons.push_str("];\n");
    }
    std::fs::write(Path::new(&out_dir).join("icons.rs"), icons)
        .unwrap_or_else(|e| panic!("write icons.rs: {e}"));
}

fn minify_json(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let (mut in_str, mut escaped) = (false, false);
    for c in src.chars() {
        if in_str {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if !c.is_ascii_whitespace() {
            out.push(c);
        }
    }
    out
}
