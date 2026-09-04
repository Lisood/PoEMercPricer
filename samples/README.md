Mercenary inspect screenshots used as test fixtures (`tests/`, `src/vision.rs`).

| File | What it covers |
|---|---|
| `manyshot_alara.png` | Cropped panel, Infamous Manyshot; support identity/tier acceptance |
| `blade_ambusher_sid.png` | Cropped panel, Blade Ambusher |
| `frosthand_secha.jpg` | Cropped panel, Frosthand |
| `storming_zealot_orvan.jpg` | Cropped panel, Storming Zealot (JPEG) |
| `sanguimancer_danalla.jpg` | Cropped panel, Sanguimancer; exact-row reference |
| `fullscreen_danalla.jpg` | Full-screen capture with an equipment tooltip overlapping the panel |
| `grynelle_sanguimancer.png` | 2560x1440 full-screen capture; every visible support cell |
| `kryxon_bladebitter.png` | 2560x1440 full-screen capture; class-header false positive |
| `colton_trarthus_supports.png` | Support-row crop for a wrapped two-line skill name |
| `kestel_bladebitter.png` | 2560x1440 full-screen capture; Return gem with almost no gold in its cell |
| `cruel_mistress_lazeth.png` | 2560x1440 full-screen capture; first support column survives a right-shifted panel box |

Scan one and print the verdict to the console:

```powershell
cargo run --release -- dump-scan samples\manyshot_alara.png
```

`--scan` takes the same path but opens the overlay window instead of printing,
so it is not usable from a script.

Adding a fixture: crop it to the mercenary panel, or to the whole screen when
the test is about full-screen geometry. Leave out chat, the friends list,
guild and account names; these files are public.
