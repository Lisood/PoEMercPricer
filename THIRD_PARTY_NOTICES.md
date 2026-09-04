# Third-party notices

## Grinding Gear Games Limited and Path of Exile

PoEMercPricer is an unofficial, non-commercial community project for Path of
Exile.

Path of Exile and all associated game content, names, game data, text,
graphics, images, skill and support icons, artwork, screenshots, and other
intellectual property are owned by or licensed to Grinding Gear Games Limited.
All rights are reserved by Grinding Gear Games Limited and its respective
licensors.

This product isn't affiliated with or endorsed by Grinding Gear Games in any
way.

The repository contains or references Path of Exile-derived material in the
following locations:

- `assets/icons/skills/` and `assets/icons/supports/` contain skill and support
  icon reference images used for local visual recognition and display.
- `assets/catalog-3.29.json` and `assets/trade-stats-3.29.json` contain
  Path of Exile-derived names, catalog information, and official trade-stat
  identifiers.
- `assets/warrant-prices-3.29.json` contains prices and listing counts
  observed on the official trade site, keyed by Path of Exile family and
  support names. It carries no seller or account names.
- `samples/` contains screenshots of Path of Exile gameplay used as OCR and
  visual-recognition fixtures.
- Documentation may quote or describe Path of Exile names, mechanics, and
  game data for identification and interoperability.

Copies of the icon artwork and portions of the catalog were retrieved through
[PoEDB](https://poedb.tw/us/Mercenaries). PoEDB is credited as the retrieval
and data-reference source only; this provenance statement does not attribute
ownership of Path of Exile material to PoEDB or transfer any rights from
Grinding Gear Games Limited.

Official trade-stat identifiers were sourced from Grinding Gear Games'
Path of Exile trade services. Use of Path of Exile materials and services
remains subject to the current
[Path of Exile Terms of Use](https://www.pathofexile.com/legal/terms-of-use-and-privacy-policy)
and applicable Grinding Gear Games policies.

## Rust crates

The executable statically links Rust crates (egui, eframe, wgpu, windows-rs,
arboard, image, self-replace and their dependencies), overwhelmingly under MIT
and/or Apache-2.0, with a handful under the Unicode, Boost, ISC, zlib, CC0 and
MPL-2.0 licences.

The full per-crate notice, with every licence text, ships as
`THIRD_PARTY_NOTICES.html` on every
[GitHub release](https://github.com/Lisood/PoEMercPricer/releases), next to the
exe. `release.yml` generates it from that tag's `Cargo.lock`, so it always
matches the exe published beside it.

To regenerate it locally:

```
cargo install cargo-about --locked --features cli
cargo about generate about.hbs -o THIRD_PARTY_NOTICES.html
```

`about.toml` holds the accepted-licence list and pins the target to
`x86_64-pc-windows-msvc` so Linux-only crates are excluded; `about.hbs` is the
template. The generated HTML is not tracked in the repository.

The bundled `assets/fonts/Ubuntu-Light.ttf` is used under the Ubuntu Font
Licence 1.0, reproduced in `assets/fonts/UFL.txt`.

## Licence boundary

The repository's MIT License covers only original PoEMercPricer source code
and documentation authored by PoEMercPricer contributors. It does not cover
the Path of Exile-derived materials identified above. PoEMercPricer and its
contributors do not grant permission to reproduce, modify, publish,
distribute, sublicense, sell, or otherwise exploit Grinding Gear Games
material.

This notice is an attribution and licence clarification. It does not state or
imply that Grinding Gear Games has approved this project or granted additional
permission to use or distribute its intellectual property.
