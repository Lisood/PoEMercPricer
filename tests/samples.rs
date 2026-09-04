use std::path::Path;

#[test]
fn app_icon_assets_have_expected_sizes_and_transparency() {
    let sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256, 512, 1024];
    for size in sizes {
        let path = format!("assets/branding/icons/app-icon-{size}.png");
        let image = image::open(&path).unwrap_or_else(|error| panic!("decode {path}: {error}"));
        assert_eq!(image.width(), size, "wrong width for {path}");
        assert_eq!(image.height(), size, "wrong height for {path}");
        let rgba = image.into_rgba8();
        assert!(
            rgba.pixels().any(|pixel| pixel[3] == 0),
            "{path} has no transparent background"
        );
        assert!(
            rgba.pixels().any(|pixel| pixel[3] == 255),
            "{path} has no opaque artwork"
        );
    }

    let master = image::open("assets/branding/app-icon-master.png").expect("decode icon master");
    assert_eq!(master.width(), master.height());
    assert!(master.width() >= 1024);

    let ico = std::fs::read("assets/branding/app-icon.ico").expect("read Windows icon");
    assert_eq!(&ico[..4], &[0, 0, 1, 0], "invalid ICO header");
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    assert_eq!(count, 9);
    let dimensions = (0..count)
        .map(|index| {
            let offset = 6 + index * 16;
            let width = if ico[offset] == 0 {
                256
            } else {
                u16::from(ico[offset])
            };
            let height = if ico[offset + 1] == 0 {
                256
            } else {
                u16::from(ico[offset + 1])
            };
            (width, height)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        dimensions,
        [
            (16, 16),
            (20, 20),
            (24, 24),
            (32, 32),
            (40, 40),
            (48, 48),
            (64, 64),
            (128, 128),
            (256, 256),
        ]
    );
}

#[test]
fn sample_screenshots_exist_and_decode() {
    let files = [
        "samples/manyshot_alara.png",
        "samples/blade_ambusher_sid.png",
        "samples/frosthand_secha.jpg",
        "samples/storming_zealot_orvan.jpg",
        "samples/sanguimancer_danalla.jpg",
        "samples/fullscreen_danalla.jpg",
        "samples/kryxon_bladebitter.png",
        "samples/grynelle_sanguimancer.png",
    ];
    for file in files {
        let path = Path::new(file);
        assert!(path.exists(), "missing {file}");
        let img = image::open(path).unwrap_or_else(|e| panic!("decode {file}: {e}"));
        assert!(img.width() > 200 && img.height() > 200, "{file} too small");
    }
}

#[test]
fn bundled_329_icon_catalog_is_complete_and_decodable() {
    let text = std::fs::read_to_string("assets/catalog-3.29.json").expect("3.29 catalog");
    let catalog: serde_json::Value = serde_json::from_str(&text).expect("valid catalog JSON");
    assert_eq!(catalog["patch"], "3.29.3");
    assert_eq!(catalog["builds"].as_array().unwrap().len(), 36);
    assert!(catalog["skills"].as_array().unwrap().len() >= 250);

    for (kind, dir) in [
        ("skills", "assets/icons/skills"),
        ("supports", "assets/icons/supports"),
    ] {
        for entry in catalog[kind].as_array().unwrap() {
            let icon = entry["icon"].as_str().expect("icon filename");
            let path = Path::new(dir).join(icon);
            assert!(path.exists(), "missing {}", path.display());
            let image = image::open(&path)
                .unwrap_or_else(|error| panic!("decode {}: {error}", path.display()));
            assert!(image.width() >= 32 && image.height() >= 32);
        }
    }
}

#[test]
fn embedded_json_is_minified_but_equivalent_to_the_source_files() {
    let pairs: [(&str, &str); 3] = [
        (
            "assets/catalog-3.29.json",
            include_str!(concat!(env!("OUT_DIR"), "/catalog-3.29.json")),
        ),
        (
            "assets/trade-stats-3.29.json",
            include_str!(concat!(env!("OUT_DIR"), "/trade-stats-3.29.json")),
        ),
        (
            "assets/warrant-prices-3.29.json",
            include_str!(concat!(env!("OUT_DIR"), "/warrant-prices-3.29.json")),
        ),
    ];
    for (path, embedded) in pairs {
        let raw = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let raw: serde_json::Value = serde_json::from_str(&raw).expect("source JSON parses");
        let min: serde_json::Value = serde_json::from_str(embedded).expect("embedded JSON parses");
        assert_eq!(raw, min, "{path}: minified copy changed the data");
        assert!(
            !embedded.contains('\n'),
            "{path}: embedded copy still carries newlines"
        );
    }
}

/// The exe embeds only Ubuntu-Light (egui's other bundled fonts were dropped
/// for size). Every non-ASCII character that appears in a string literal under
/// src/ must have a glyph in it, otherwise the UI would render a "?".
#[test]
fn embedded_font_covers_every_non_ascii_character_used_in_src() {
    fn collect(dir: &Path, out: &mut std::collections::BTreeSet<char>) {
        for entry in std::fs::read_dir(dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = std::fs::read_to_string(&path).expect("read source file");
                out.extend(text.chars().filter(|c| !c.is_ascii()));
            }
        }
    }
    let mut chars = std::collections::BTreeSet::new();
    collect(Path::new("src"), &mut chars);
    let text: String = chars.iter().collect();
    assert!(!text.is_empty(), "expected some non-ASCII UI text in src/");

    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert(
        "ubuntu_light".into(),
        egui::FontData::from_owned(std::fs::read("assets/fonts/Ubuntu-Light.ttf").expect("font"))
            .into(),
    );
    fonts
        .families
        .insert(egui::FontFamily::Proportional, vec!["ubuntu_light".into()]);
    fonts
        .families
        .insert(egui::FontFamily::Monospace, vec!["ubuntu_light".into()]);
    let ctx = egui::Context::default();
    ctx.set_fonts(fonts);
    let _ = ctx.run(Default::default(), |ctx| {
        ctx.fonts(|f| {
            let id = egui::FontId::proportional(14.0);
            // Negative control: Ubuntu-Light has no emoji, so has_glyphs must be able to say no.
            assert!(
                !f.has_glyphs(&id, "\u{1F600}"),
                "has_glyphs cannot detect a missing glyph"
            );
            let missing: String = text
                .chars()
                .filter(|c| !f.has_glyphs(&id, &c.to_string()))
                .collect();
            assert!(
                missing.is_empty(),
                "Ubuntu-Light lacks glyphs for {missing:?}; add a font or change the text"
            );
        });
    });
}
