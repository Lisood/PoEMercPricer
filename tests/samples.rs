use std::path::Path;

#[test]
fn sample_screenshots_exist_and_decode() {
    let files = [
        "samples/manyshot_alara.png",
        "samples/blade_ambusher_sid.png",
        "samples/frosthand_secha.jpg",
        "samples/storming_zealot_orvan.jpg",
        "samples/sanguimancer_danalla.jpg",
        "samples/fullscreen_danalla.jpg",
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
