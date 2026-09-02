//! Layout + icon vision for the mercenary inspect panel.
//!
//! Skill *names* are cream text (OCR). Support gems are gold-framed icons with
//! a roman-numeral tier I/II/III in the corner — those are the gem "levels".

use std::path::PathBuf;
use std::sync::OnceLock;

use image::RgbaImage;

use crate::catalog::{compatible_supports_for_skill_tier, parse_tier, support_display_for_tier};
use crate::models::{SupportGem, SupportTier};
use crate::winocr::{recognize_lines, OcrLine};

struct SupportTemplate {
    // Multiple catalog art ids can be byte-identical (all generic gold arts,
    // for example). Keep their identities together so compatibility can
    // narrow them without ever inventing a name from indistinguishable pixels.
    art_keys: Vec<String>,
    fine: RgbaImage,
    coarse: RgbaImage,
}

fn support_templates() -> &'static [SupportTemplate] {
    static T: OnceLock<Vec<SupportTemplate>> = OnceLock::new();
    T.get_or_init(|| {
        let mut loaded: Vec<SupportTemplate> = Vec::new();
        let mut seen_art_keys = std::collections::HashSet::new();
        for dir in template_dirs() {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    if !matches!(p.extension().and_then(|e| e.to_str()), Some("webp")) {
                        continue;
                    }
                    let stem = p.file_stem().unwrap().to_string_lossy();
                    let art_key = stem
                        .split_once("__")
                        .map(|(_, art)| art)
                        .unwrap_or(&stem)
                        .to_ascii_lowercase();
                    if !seen_art_keys.insert(art_key.clone()) {
                        continue;
                    }
                    if let Ok(img) = image::open(&p) {
                        let fine = resize_nn(&img.to_rgba8(), 32, 32);
                        // Collapse equal pixels for speed, but retain every art
                        // identity for skill-compatibility disambiguation.
                        if let Some(equal) = loaded.iter_mut().find(|known| known.fine == fine) {
                            equal.art_keys.push(art_key);
                        } else {
                            loaded.push(SupportTemplate {
                                art_keys: vec![art_key],
                                coarse: resize_nn(&fine, 16, 16),
                                fine,
                            });
                        }
                    }
                }
            }
        }
        loaded
    })
}

fn template_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons/supports"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("assets/icons/supports"));
            dirs.push(parent.join("../assets/icons/supports"));
        }
    }
    dirs
}

#[derive(Debug)]
struct IconMatch {
    canonical: String,
    name: String,
    confidence: f32,
}

fn compatible_candidates(
    template: &SupportTemplate,
    skill_name: &str,
    tier: u8,
) -> Vec<(&'static str, &'static str)> {
    let mut candidates: Vec<(&'static str, &'static str)> = Vec::new();
    for art_key in &template.art_keys {
        for candidate in compatible_supports_for_skill_tier(art_key, skill_name, tier) {
            if !candidates
                .iter()
                .any(|(canonical, _)| canonical.eq_ignore_ascii_case(candidate.0))
            {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_unstable_by(|a, b| a.0.cmp(b.0));
    candidates
}

fn match_support_icon(
    img: &RgbaImage,
    cell: (u32, u32, u32, u32),
    skill_name: &str,
    tier: u8,
) -> Option<IconMatch> {
    let variants = normalized_cell_variants(img, cell);
    let coarse_variants: Vec<_> = variants
        .iter()
        .map(|variant| resize_nn(variant, 16, 16))
        .collect();
    let templates = support_templates();
    let mut shortlist: Vec<(usize, f32)> = templates
        .iter()
        .enumerate()
        .filter(|(_, template)| !compatible_candidates(template, skill_name, tier).is_empty())
        .map(|(index, template)| {
            let score = coarse_variants
                .iter()
                .map(|variant| masked_ncc_shifted(variant, &template.coarse, 0, 0))
                .max_by(f32::total_cmp)
                .unwrap_or(0.0);
            (index, score)
        })
        .collect();
    shortlist.sort_by(|a, b| b.1.total_cmp(&a.1));
    shortlist.truncate(8);

    let mut ranked: Vec<(usize, f32)> = shortlist
        .into_iter()
        .map(|(index, _)| {
            let template = &templates[index];
            let score = (-2..=2)
                .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| icon_similarity(&variants[1], &template.fine, dx, dy))
                .max_by(f32::total_cmp)
                .unwrap_or(0.0);
            (index, score)
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    // Only search alternate crop scales when the normal cell boundary is not
    // already a confident match. Letting every template choose a different
    // scale overfits clean captures and creates artificial ties.
    let normal_margin = ranked
        .first()
        .zip(ranked.get(1))
        .map(|((_, best), (_, runner))| best - runner)
        .unwrap_or(1.0);
    if ranked
        .first()
        .map(|(_, score)| *score < 0.60 || normal_margin < 0.01)
        .unwrap_or(false)
    {
        for (index, score) in &mut ranked {
            let template = &templates[*index];
            *score = variants
                .iter()
                .flat_map(|variant| {
                    (-2..=2).flat_map(move |dy| {
                        (-2..=2).map(move |dx| icon_similarity(variant, &template.fine, dx, dy))
                    })
                })
                .max_by(f32::total_cmp)
                .unwrap_or(0.0);
        }
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    }
    let (index, score) = *ranked.first()?;
    if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
        let runner = ranked.get(1).map(|(_, value)| *value).unwrap_or(0.0);
        eprintln!(
            "  icon {skill_name} @{},{}: best={score:.3} runner={runner:.3} candidates={:?}",
            cell.0,
            cell.1,
            compatible_candidates(&templates[index], skill_name, tier)
        );
    }
    // Compatibility makes each skill's search space small. A moderately
    // degraded but decisive match is more useful than an unnamed placeholder.
    // Below the conservative exact threshold, retain the three best
    // catalog-backed candidate groups instead of inventing one identity.
    if score < 0.42 {
        let mut low_confidence: Vec<(&'static str, &'static str)> = Vec::new();
        for (candidate_index, _) in ranked.iter().take(3) {
            for candidate in compatible_candidates(&templates[*candidate_index], skill_name, tier) {
                if !low_confidence
                    .iter()
                    .any(|(canonical, _)| canonical.eq_ignore_ascii_case(candidate.0))
                {
                    low_confidence.push(candidate);
                }
            }
        }
        low_confidence.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let name = low_confidence
            .iter()
            .map(|(_, name)| *name)
            .collect::<Vec<_>>()
            .join(" / ");
        return (!name.is_empty()).then_some(IconMatch {
            canonical: "ambiguous".into(),
            name,
            confidence: score.clamp(0.0, 1.0),
        });
    }
    let mut candidates = compatible_candidates(&templates[index], skill_name, tier);
    // Distinct arts can become visually indistinguishable after capture,
    // scaling, and compression. Preserve both catalog-backed candidates when
    // their measured scores are effectively tied instead of asserting the
    // wrong exact identity.
    if let Some((runner_index, runner_score)) = ranked.get(1).copied() {
        // Alternate-scale refinement has already handled ordinary crop
        // uncertainty. Only preserve a distinct-art runner when its pixels are
        // effectively identical; a wider tie band incorrectly merged the red
        // Added Fire art with the silver Physical as Extra art in live capture.
        if score - runner_score < 0.001 {
            for candidate in compatible_candidates(&templates[runner_index], skill_name, tier) {
                if !candidates
                    .iter()
                    .any(|(canonical, _)| canonical.eq_ignore_ascii_case(candidate.0))
                {
                    candidates.push(candidate);
                }
            }
            candidates.sort_unstable_by(|a, b| a.0.cmp(b.0));
        }
    }
    if candidates.len() == 1 {
        return Some(IconMatch {
            canonical: candidates[0].0.to_owned(),
            name: candidates[0].1.to_owned(),
            confidence: score.clamp(0.0, 1.0),
        });
    }

    // Identical pixels can legitimately represent several mercenary-specific
    // supports. Preserve every compatible name, rather than returning `gem` or
    // making a financially consequential guess.
    let name = candidates
        .iter()
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(" / ");
    Some(IconMatch {
        canonical: "ambiguous".into(),
        name,
        confidence: score.clamp(0.0, 1.0),
    })
}

/// Normalize modest UI-scale/cell-boundary differences before matching. PoE's
/// icon pixels are the same, but DPI and OCR row bounds can make the estimated
/// square 10–15% too tight or too loose.
fn normalized_cell_variants(img: &RgbaImage, cell: (u32, u32, u32, u32)) -> Vec<RgbaImage> {
    let (x, y, w, h) = cell;
    let cx = x + w / 2;
    let cy = y + h / 2;
    [85_u32, 100, 115]
        .into_iter()
        .map(|percent| {
            let scaled_w = (w * percent / 100).clamp(16, img.width());
            let scaled_h = (h * percent / 100).clamp(16, img.height());
            let left = cx.saturating_sub(scaled_w / 2);
            let top = cy.saturating_sub(scaled_h / 2);
            resize_nn(&crop(img, left, top, scaled_w, scaled_h), 32, 32)
        })
        .collect()
}

fn icon_similarity(a: &RgbaImage, b: &RgbaImage, dx: i32, dy: i32) -> f32 {
    // NCC handles lighting/contrast changes; absolute colour similarity keeps
    // similarly shaped but differently coloured arts from tying it.
    0.65 * masked_ncc_shifted(a, b, dx, dy) + 0.35 * masked_mae_similarity_shifted(a, b, dx, dy)
}

fn masked_mae_similarity_shifted(a: &RgbaImage, b: &RgbaImage, dx: i32, dy: i32) -> f32 {
    if a.dimensions() != b.dimensions() {
        return 0.0;
    }
    let (w, h) = a.dimensions();
    let mut count = 0u32;
    let mut error = 0u64;
    for y in 0..h {
        for x in 0..w {
            let ax = x as i32 + dx;
            let ay = y as i32 + dy;
            if ax < 0 || ay < 0 || ax >= w as i32 || ay >= h as i32 {
                continue;
            }
            if ax as u32 >= w * 2 / 5 && ay as u32 >= h * 11 / 20 {
                continue;
            }
            let pb = b.get_pixel(x, y).0;
            if pb[3] < 24 || pb[..3].iter().copied().max().unwrap_or(0) < 10 {
                continue;
            }
            let pa = a.get_pixel(ax as u32, ay as u32).0;
            count += 1;
            for c in 0..3 {
                error += pa[c].abs_diff(pb[c]) as u64;
            }
        }
    }
    if count < 32 {
        return 0.0;
    }
    1.0 - error as f32 / (count as f32 * 3.0 * 255.0)
}

fn resize_nn(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let sx = x * img.width() / w;
            let sy = y * img.height() / h;
            out.put_pixel(
                x,
                y,
                *img.get_pixel(sx.min(img.width() - 1), sy.min(img.height() - 1)),
            );
        }
    }
    out
}

fn masked_ncc_shifted(a: &RgbaImage, b: &RgbaImage, dx: i32, dy: i32) -> f32 {
    if a.dimensions() != b.dimensions() {
        return 0.0;
    }
    let (w, h) = a.dimensions();
    let mut count = 0u32;
    let mut sum_a = [0.0f64; 3];
    let mut sum_b = [0.0f64; 3];
    let mut sum_aa = [0.0f64; 3];
    let mut sum_bb = [0.0f64; 3];
    let mut sum_ab = [0.0f64; 3];
    for y in 0..h {
        for x in 0..w {
            let ax = x as i32 + dx;
            let ay = y as i32 + dy;
            if ax < 0 || ay < 0 || ax >= w as i32 || ay >= h as i32 {
                continue;
            }
            // The game draws the support tier over this part of the icon.
            if ax as u32 >= w * 2 / 5 && ay as u32 >= h * 11 / 20 {
                continue;
            }
            let pb = b.get_pixel(x, y).0;
            if pb[3] < 24 || pb[..3].iter().copied().max().unwrap_or(0) < 10 {
                continue;
            }
            let pa = a.get_pixel(ax as u32, ay as u32).0;
            count += 1;
            for c in 0..3 {
                let va = pa[c] as f64;
                let vb = pb[c] as f64;
                sum_a[c] += va;
                sum_b[c] += vb;
                sum_aa[c] += va * va;
                sum_bb[c] += vb * vb;
                sum_ab[c] += va * vb;
            }
        }
    }
    if count < 32 {
        return 0.0;
    }
    let n = count as f64;
    let mut num = 0.0f64;
    let mut da = 0.0f64;
    let mut db = 0.0f64;
    for c in 0..3 {
        num += sum_ab[c] - sum_a[c] * sum_b[c] / n;
        da += sum_aa[c] - sum_a[c] * sum_a[c] / n;
        db += sum_bb[c] - sum_b[c] * sum_b[c] / n;
    }
    let den = (da * db).sqrt();
    if den < 1e-3 {
        0.0
    } else {
        (num / den) as f32
    }
}

#[derive(Clone, Debug)]
pub struct SupportCell {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub tier: SupportTier,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub fn guess_panel(img: &RgbaImage, lines: &[OcrLine]) -> PanelBox {
    let (iw, ih) = (img.width(), img.height());
    // Clipboard and saved panel screenshots are already tightly cropped. OCR
    // lines cover only the text column, so using them to infer the right edge
    // would cut off every support icon.
    if iw < 1400 {
        return PanelBox {
            x: 0,
            y: 0,
            w: iw,
            h: ih,
        };
    }
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for ln in lines {
        let n = ln.text.to_lowercase();
        if (n.contains("wager")
            || n.contains("rematch")
            || n.contains("recruit")
            || n.contains("take item")
            || n.contains("ice shot")
            || n.contains("lvl")
            || n.contains("lvi"))
            && ln.w > 0
        {
            xs.push(ln.x);
            xs.push(ln.x + ln.w);
            ys.push(ln.y);
            ys.push(ln.y + ln.h);
        }
    }
    if xs.len() >= 4 {
        let pad_x = iw / 12;
        let pad_y = ih / 8;
        let x0 = xs.iter().min().copied().unwrap_or(0).saturating_sub(pad_x);
        let y0 = ys.iter().min().copied().unwrap_or(0).saturating_sub(pad_y);
        let x1 = (xs.iter().max().copied().unwrap_or(iw) + pad_x).min(iw);
        let y1 = (ys.iter().max().copied().unwrap_or(ih) + pad_y).min(ih);
        if x1 > x0 + 200 && y1 > y0 + 200 {
            return PanelBox {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            };
        }
    }
    let w = (iw as f32 * 0.42) as u32;
    let h = (ih as f32 * 0.92) as u32;
    PanelBox {
        x: (iw - w) / 2,
        y: (ih as f32 * 0.04) as u32,
        w,
        h,
    }
}

pub fn find_support_cells(img: &RgbaImage, row_y: u32, row_h: u32) -> Vec<(u32, u32, u32, u32)> {
    find_support_cells_between(img, row_y, row_h, img.width() * 48 / 100, img.width())
}

fn find_support_cells_between(
    img: &RgbaImage,
    row_y: u32,
    row_h: u32,
    x_min: u32,
    x_max: u32,
) -> Vec<(u32, u32, u32, u32)> {
    let (iw, ih) = (img.width(), img.height());
    let y0 = row_y.min(ih.saturating_sub(1));
    let y1 = (row_y + row_h).min(ih);
    let x_min = x_min.min(iw);
    let x_max = x_max.min(iw);
    if y1 <= y0 || x_max <= x_min {
        return Vec::new();
    }
    let mw = x_max - x_min;
    let mut mask = vec![false; ((y1 - y0) * mw) as usize];
    for y in y0..y1 {
        for x in x_min..x_max {
            let p = img.get_pixel(x, y).0;
            if is_gold_frame(p[0], p[1], p[2]) {
                mask[((y - y0) * mw + (x - x_min)) as usize] = true;
            }
        }
    }
    let mut visited = vec![false; mask.len()];
    let side = (row_h * 6 / 7).clamp(24, 64).min(iw).min(ih);
    let mut fragments: Vec<(u32, u32, u32)> = Vec::new();
    let h = y1 - y0;
    for i in 0..mask.len() {
        if !mask[i] || visited[i] {
            continue;
        }
        let (minx, miny, maxx, maxy, area) = flood(&mask, &mut visited, mw, h, i);
        let cw = maxx - minx + 1;
        let ch = maxy - miny + 1;
        if area < 8 || ch > side * 3 / 2 {
            continue;
        }
        // Long, one-pixel strokes are the inspect-panel row separators.
        if ch <= 3 {
            continue;
        }

        if cw > side * 3 / 2 {
            // Adjacent icon frames occasionally become one connected component
            // after capture scaling or sharpening adds a hairline gold bridge.
            // Recover the regular support-cell grid from the component span. A
            // real cell is roughly `side` wide and adjacent centers are about
            // 6/5 of `side` apart in the in-game panel.
            let pitch = (side * 6 / 5).max(1);
            let gutter = pitch.saturating_sub(side);
            let count = ((cw + gutter + pitch / 2) / pitch).clamp(2, 8);
            let expected_span = side + (count - 1) * pitch;
            let span_error = cw.abs_diff(expected_span);
            let looks_like_joined_cells =
                ch >= side / 2 && span_error <= side / 2 && area >= side.saturating_mul(count);
            if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
                eprintln!(
                    "  wide gold component x={} y={} w={} h={} area={} count={} error={} split={}",
                    x_min + minx,
                    y0 + miny,
                    cw,
                    ch,
                    area,
                    count,
                    span_error,
                    looks_like_joined_cells
                );
            }
            if looks_like_joined_cells {
                let component_center = minx + (cw - 1) / 2;
                let grid_span = (count - 1) * pitch;
                let first_center = component_center.saturating_sub(grid_span / 2);
                let fragment_half = (side / 3).max(1);
                for index in 0..count {
                    let center = first_center + index * pitch;
                    fragments.push((
                        x_min + center.saturating_sub(fragment_half),
                        x_min + center.saturating_add(fragment_half),
                        area / count,
                    ));
                }
            }
            continue;
        }
        fragments.push((x_min + minx, x_min + maxx, area));
    }
    fragments.sort_by_key(|fragment| fragment.0);

    // Gold artwork and frames are often disconnected. Cluster their horizontal
    // fragments into cells rather than requiring one square connected component.
    let join_gap = (side / 6).max(3);
    let mut clusters: Vec<(u32, u32, u32)> = Vec::new();
    for (left, right, area) in fragments {
        if let Some(last) = clusters.last_mut() {
            if left <= last.1.saturating_add(join_gap) {
                last.1 = last.1.max(right);
                last.2 += area;
                continue;
            }
        }
        clusters.push((left, right, area));
    }

    let clusters: Vec<_> = clusters
        .into_iter()
        .filter(|(left, right, area)| {
            let span = right - left + 1;
            *area >= side * 2 && span >= side / 3 && span <= side * 3 / 2
        })
        .collect();

    // The OCR line height is a useful upper bound, but full-screen captures
    // render the support cells smaller than tightly cropped panel screenshots.
    // Derive the square from the observed cell pitch so a 38-40 px icon is not
    // compared together with pixels from a nominal 48 px row estimate.  The
    // cap preserves the proven cropped-panel geometry, while the lower bound
    // prevents one noisy fragment from shrinking every cell in the row.
    let centers: Vec<u32> = clusters
        .iter()
        .map(|(left, right, _)| left + (right - left) / 2)
        .collect();
    let detected_side = if centers.len() >= 2 {
        let mut pitches: Vec<u32> = centers
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]))
            .filter(|pitch| *pitch >= side / 2 && *pitch <= side * 2)
            .collect();
        pitches.sort_unstable();
        pitches
            .get(pitches.len() / 2)
            .map(|pitch| pitch * 5 / 6)
            .unwrap_or(side)
            .clamp((side * 3 / 4).max(24), side)
    } else {
        side
    };
    let cell_y = (row_y + row_h / 2)
        .saturating_sub(detected_side / 2)
        .min(ih.saturating_sub(detected_side));
    clusters
        .into_iter()
        .map(|(left, right, _)| {
            let center = left + (right - left) / 2;
            let cell_x = center
                .saturating_sub(detected_side / 2)
                .min(iw.saturating_sub(detected_side));
            (cell_x, cell_y, detected_side, detected_side)
        })
        .collect()
}

pub fn read_cell_tier(img: &RgbaImage, cell: (u32, u32, u32, u32)) -> SupportTier {
    let (x, y, w, h) = cell;
    let x0 = x + w * 5 / 12;
    let y0 = y + h * 13 / 24;
    let x1 = (x + w * 11 / 12).min(img.width());
    let y1 = (y + h * 15 / 16).min(img.height());
    if x1 <= x0 || y1 <= y0 {
        return SupportTier::T2;
    }
    // Stroke-count on bright roman numerals: I=1, II=2, III=3.
    let strokes = count_roman_strokes(img, x0, y0, x1, y1);
    match strokes {
        1 => SupportTier::T1,
        2 => SupportTier::T2,
        3 => SupportTier::T3,
        _ => {
            if let Ok(lines) = ocr_crop(img, x0, y0, x1 - x0, y1 - y0) {
                let blob = lines.join(" ");
                SupportTier::from_u8(parse_tier(&blob))
            } else {
                SupportTier::T2
            }
        }
    }
}

pub fn supports_for_skill_row(
    img: &RgbaImage,
    skill_line: &OcrLine,
    skill_name: &str,
    panel: PanelBox,
) -> Vec<SupportGem> {
    let started = std::time::Instant::now();
    let row_h = skill_line.h.max(28).saturating_mul(2).min(70);
    let cy = skill_line.y + skill_line.h / 2;
    let y0 = cy.saturating_sub(row_h / 2);
    if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
        eprintln!("  find cells y={y0} h={row_h} line={:?}", skill_line.text);
    }
    let support_x0 = panel.x.saturating_add(panel.w * 48 / 100);
    let support_x1 = panel.x.saturating_add(panel.w).min(img.width());
    let cells = find_support_cells_between(img, y0, row_h, support_x0, support_x1);
    if std::env::var_os("POEMERC_PROFILE_OCR").is_some() {
        eprintln!(
            "  cells y={} h={}: {} in {:.1} ms",
            y0,
            row_h,
            cells.len(),
            started.elapsed().as_secs_f64() * 1_000.0
        );
    }
    cells
        .into_iter()
        .map(|cell| {
            let tier = read_cell_tier(img, cell);
            let matched = match_support_icon(img, cell, skill_name, tier as u8);
            let canonical = matched
                .as_ref()
                .map(|matched| matched.canonical.clone())
                .unwrap_or_else(|| "unknown".into());
            let name = if !matches!(canonical.as_str(), "unknown" | "ambiguous") {
                support_display_for_tier(&canonical, tier as u8)
            } else {
                matched
                    .as_ref()
                    .map(|matched| matched.name.clone())
                    .unwrap_or_else(|| format!("Unresolved support T{}", tier as u8))
            };
            SupportGem {
                name,
                canonical,
                tier,
                confidence: matched
                    .as_ref()
                    .map(|matched| matched.confidence)
                    .unwrap_or(0.4),
                raw: format!("icon@{},{} T{}", cell.0, cell.1, tier as u8),
            }
        })
        .collect()
}

fn ocr_crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Result<Vec<String>, anyhow::Error> {
    let sub = crop(img, x, y, w, h);
    let up = upscale(&sub, 3);
    let lines = recognize_lines(&up)?;
    Ok(lines.into_iter().map(|l| l.text).collect())
}

pub fn crop(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let x1 = (x + w).min(img.width());
    let y1 = (y + h).min(img.height());
    let x0 = x.min(img.width());
    let y0 = y.min(img.height());
    let mut out = RgbaImage::new(x1.saturating_sub(x0).max(1), y1.saturating_sub(y0).max(1));
    for yy in y0..y1 {
        for xx in x0..x1 {
            out.put_pixel(xx - x0, yy - y0, *img.get_pixel(xx, yy));
        }
    }
    out
}

fn upscale(img: &RgbaImage, n: u32) -> RgbaImage {
    let mut out = RgbaImage::new(img.width() * n, img.height() * n);
    for y in 0..img.height() {
        for x in 0..img.width() {
            let p = *img.get_pixel(x, y);
            for dy in 0..n {
                for dx in 0..n {
                    out.put_pixel(x * n + dx, y * n + dy, p);
                }
            }
        }
    }
    out
}

fn is_gold_frame(r: u8, g: u8, b: u8) -> bool {
    let (h, s, v) = rgb_to_hsv(r, g, b);
    ((18.0..=55.0).contains(&h) || (40.0..=58.0).contains(&h)) && s >= 0.18 && v >= 0.28
}

fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-6 {
        0.0
    } else if (max - r).abs() < 1e-6 {
        60.0 * (((g - b) / d) % 6.0)
    } else if (max - g).abs() < 1e-6 {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let s = if max < 1e-6 { 0.0 } else { d / max };
    (h, s, max)
}

fn flood(
    mask: &[bool],
    visited: &mut [bool],
    w: u32,
    h: u32,
    start: usize,
) -> (u32, u32, u32, u32, u32) {
    let mut stack = vec![start];
    visited[start] = true;
    let mut minx = u32::MAX;
    let mut miny = u32::MAX;
    let mut maxx = 0;
    let mut maxy = 0;
    let mut area = 0u32;
    while let Some(i) = stack.pop() {
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
        area += 1;
        for (nx, ny) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if mask[j] && !visited[j] {
                visited[j] = true;
                stack.push(j);
            }
        }
    }
    (minx, miny, maxx, maxy, area)
}

fn count_roman_strokes(img: &RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
    let w = x1.saturating_sub(x0).max(1);
    let h = y1.saturating_sub(y0).max(1);
    let mut col_on = vec![0u32; w as usize];
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y).0;
            // Numerals are a tall ochre-gold. Requiring both red and green
            // rejects the green/blue gem artwork beneath the overlay.
            let gold = p[0] >= 105
                && p[1] >= 65
                && p[0] as u16 * 10 >= p[2] as u16 * 14
                && p[1] as u16 * 10 >= p[2] as u16 * 11;
            if gold {
                col_on[(x - x0) as usize] += 1;
            }
        }
    }
    let thresh = (h / 2).max(3);
    let mut strokes = 0u32;
    let mut in_stroke = false;
    for c in col_on {
        let on = c >= thresh;
        if on && !in_stroke {
            strokes += 1;
            in_stroke = true;
        } else if !on {
            in_stroke = false;
        }
    }
    strokes.min(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alara_support_grid_names_and_tiers_are_read_from_pixels() {
        let img = image::open("samples/manyshot_alara.png")
            .unwrap()
            .to_rgba8();
        let rows = [(742, 56), (858, 56), (916, 56), (1032, 56)];
        let expected_counts = [4, 5, 4, 3];
        let skill_names = ["Ice Shot", "Icicle Rain", "Mirror Arrow", "Blink Arrow"];
        let expected_names = [
            vec!["aoe", "cold_penetration", "fork", "hypothermia"],
            vec!["aoe", "gmp", "edwa", "fork", "hypothermia"],
            vec!["cdr", "gmp", "second_wind", "ambiguous"],
            vec!["more_duration", "faster_attacks", "faster_projectiles"],
        ];
        let expected_tiers = [
            vec![
                SupportTier::T2,
                SupportTier::T2,
                SupportTier::T3,
                SupportTier::T3,
            ],
            vec![
                SupportTier::T2,
                SupportTier::T3,
                SupportTier::T2,
                SupportTier::T3,
                SupportTier::T3,
            ],
            vec![
                SupportTier::T2,
                SupportTier::T3,
                SupportTier::T3,
                SupportTier::T2,
            ],
            vec![SupportTier::T2, SupportTier::T2, SupportTier::T2],
        ];
        for (index, (row_y, row_h)) in rows.into_iter().enumerate() {
            let cells = find_support_cells(&img, row_y, row_h);
            assert_eq!(
                cells.len(),
                expected_counts[index],
                "row {index}: {cells:?}"
            );
            let tiers: Vec<_> = cells
                .iter()
                .map(|cell| read_cell_tier(&img, *cell))
                .collect();
            assert_eq!(tiers, expected_tiers[index], "row {index}: {cells:?}");
            let matches: Vec<_> = cells
                .iter()
                .map(|cell| {
                    match_support_icon(
                        &img,
                        *cell,
                        skill_names[index],
                        read_cell_tier(&img, *cell) as u8,
                    )
                    .expect("every visible support must be identified")
                })
                .collect();
            let names: Vec<_> = matches
                .iter()
                .map(|matched| matched.canonical.as_str())
                .collect();
            assert_eq!(names, expected_names[index], "row {index}: {matches:?}");
        }

        let ice = find_support_cells(&img, rows[0].0, rows[0].1);
        let matches: Vec<_> = ice
            .iter()
            .map(|cell| {
                match_support_icon(&img, *cell, "Ice Shot", read_cell_tier(&img, *cell) as u8)
                    .unwrap()
            })
            .collect();
        let names: Vec<_> = matches
            .into_iter()
            .map(|matched| matched.canonical)
            .collect();
        assert_eq!(names, ["aoe", "cold_penetration", "fork", "hypothermia"]);
        let mirror = find_support_cells(&img, rows[2].0, rows[2].1);
        let second_wind = match_support_icon(
            &img,
            mirror[2],
            "Mirror Arrow",
            read_cell_tier(&img, mirror[2]) as u8,
        )
        .unwrap();
        let second_cell = mirror[2];
        let sized = resize_nn(
            &crop(
                &img,
                second_cell.0,
                second_cell.1,
                second_cell.2,
                second_cell.3,
            ),
            32,
            32,
        );
        let mut runner_scores: Vec<_> = support_templates()
            .iter()
            .filter(|template| !compatible_candidates(template, "Mirror Arrow", 3).is_empty())
            .map(|template| {
                let score = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| icon_similarity(&sized, &template.fine, dx, dy))
                    .max_by(f32::total_cmp)
                    .unwrap_or(0.0);
                (score, &template.art_keys)
            })
            .collect();
        runner_scores.sort_by(|a, b| b.0.total_cmp(&a.0));
        assert_eq!(second_wind.canonical, "second_wind");
        assert!(second_wind.confidence >= 0.50);
        assert!(runner_scores[0].0 - runner_scores[1].0 >= 0.01);
        let ambiguous = match_support_icon(
            &img,
            mirror[3],
            "Mirror Arrow",
            read_cell_tier(&img, mirror[3]) as u8,
        )
        .unwrap();
        assert_eq!(ambiguous.canonical, "ambiguous");
        assert_eq!(ambiguous.name, "Minion Damage / Minion Life");
        assert!(support_templates().len() <= 96, "runtime template budget");
    }

    #[test]
    fn colton_trarthus_supports_are_read_from_stable_pixel_crop() {
        let img = image::open("samples/colton_trarthus_supports.png")
            .expect("decode stable Colton support-row crop")
            .to_rgba8();
        // The two-line skill label is represented by its unioned OCR bounds.
        // Keeping the full row height is essential: the top-only line clips the
        // roman-numeral overlay and makes visible III supports look like I.
        let cells = find_support_cells_between(&img, 13, 70, 0, img.width());
        assert_eq!(
            cells.len(),
            2,
            "Wave of Conviction support cells: {cells:?}"
        );

        let matches: Vec<_> = cells
            .iter()
            .map(|cell| {
                match_support_icon(&img, *cell, "Wave of Conviction of Trarthus", 3)
                    .expect("identify attached support icon")
            })
            .collect();
        assert_eq!(
            matches
                .iter()
                .map(|matched| matched.canonical.as_str())
                .collect::<Vec<_>>(),
            ["added_fire", "aoe"],
            "attached icons must resolve to their exact catalog artwork: {matches:?}"
        );
        assert_eq!(
            cells
                .iter()
                .map(|cell| read_cell_tier(&img, *cell))
                .collect::<Vec<_>>(),
            [SupportTier::T3, SupportTier::T3],
            "both attached supports visibly use the III overlay: {cells:?}"
        );
    }

    #[test]
    fn support_grid_survives_a_one_pixel_bridge_between_adjacent_frames() {
        let mut img = image::open("samples/grynelle_sanguimancer.png")
            .expect("decode Grynnelle regression capture")
            .to_rgba8();
        let row_y = 793;
        let row_h = 56;
        // Limit detection to the mercenary panel; the full-screen capture also
        // contains gold HUD elements to the right of it.
        let x_min = 1_200;
        let x_max = 1_640;
        let clean = find_support_cells_between(&img, row_y, row_h, x_min, x_max);
        assert_eq!(clean.len(), 3, "fixture must contain all three supports");

        // Capture scaling and GPU sharpening can turn the narrow dark gutter
        // between two gold frames into a one-pixel bridge.  The bridge must not
        // collapse two visible support cells into one detection component.
        let y = clean[0].1 + clean[0].3 / 2;
        let left_center = clean[0].0 + clean[0].2 / 2;
        let right_center = clean[1].0 + clean[1].2 / 2;
        for x in left_center..=right_center {
            img.put_pixel(x, y, image::Rgba([190, 140, 45, 255]));
        }

        let bridged = find_support_cells_between(&img, row_y, row_h, x_min, x_max);
        assert_eq!(
            bridged.len(),
            3,
            "a fragile gold bridge must not merge adjacent cells: {bridged:?}"
        );
        for (expected, actual) in clean.iter().zip(&bridged) {
            assert!(
                expected.0.abs_diff(actual.0) <= 2,
                "recovered crop shifted away from its icon: {clean:?} vs {bridged:?}"
            );
        }
        assert_eq!(
            bridged
                .iter()
                .map(|cell| {
                    match_support_icon(
                        &img,
                        *cell,
                        "Boiling Blood",
                        read_cell_tier(&img, *cell) as u8,
                    )
                    .expect("every recovered support cell must remain identifiable")
                    .canonical
                })
                .collect::<Vec<_>>(),
            ["brutality", "dot_multiplier", "more_duration"]
        );
    }
}
