//! Box drawing and block characters drawn as shapes that fill their cell
//! exactly, as Ghostty does. Font glyphs for these rarely span the whole
//! cell, so tables get gaps between rows and block art gets seams.

use gpui::{Bounds, Hsla, PathBuilder, Pixels, Window, fill, point, px, size};

/// Per direction (up, right, down, left): 0 none, 1 light, 2 heavy, 3 double.
fn lines(c: char) -> Option<[u8; 4]> {
    const LIGHT_HEAVY: &str = "\
        0101 0202 1010 2020 0101 0202 1010 2020 0101 0202 1010 2020 \
        0110 0210 0120 0220 0011 0012 0021 0022 1100 1200 2100 2200 \
        1001 1002 2001 2002 1110 1210 2110 1120 2120 2210 1220 2220 \
        1011 1012 2011 1021 2021 2012 1022 2022 0111 0112 0211 0212 \
        0121 0122 0221 0222 1101 1102 1201 1202 2101 2102 2201 2202 \
        1111 1112 1211 1212 2111 1121 2121 2112 2211 1122 1221 2212 \
        1222 2122 2221 2222 0101 0202 1010 2020";
    const DOUBLE: &str = "\
        0303 3030 0310 0130 0330 0013 0031 0033 1300 3100 3300 1003 \
        3001 3003 1310 3130 3330 1013 3031 3033 0313 0131 0333 1303 \
        3101 3303 1313 3131 3333 0110 0011 1001 1100";
    const SINGLES: &str = "0001 1000 0100 0010 0002 2000 0200 0020 0201 1020 0102 2010";
    let code = c as u32;
    let (table, base) = match code {
        0x2500..=0x254F => (LIGHT_HEAVY, 0x2500),
        0x2550..=0x2570 => (DOUBLE, 0x2550),
        0x2574..=0x257F => (SINGLES, 0x2574),
        _ => return None,
    };
    let word = table.split_whitespace().nth((code - base) as usize)?;
    let mut out = [0u8; 4];
    for (o, b) in out.iter_mut().zip(word.bytes()) {
        *o = b - b'0';
    }
    Some(out)
}

/// Rectangles in cell fractions (x0, y0, x1, y1), each with an alpha: block
/// elements and the block mosaics of Symbols for Legacy Computing.
fn blocks(c: char) -> Option<Vec<([f32; 4], f32)>> {
    if let Some((rects, alpha)) = block_elements(c) {
        return Some(rects.into_iter().map(|r| (r, alpha)).collect());
    }
    legacy_blocks(c as u32)
}

fn block_elements(c: char) -> Option<(Vec<[f32; 4]>, f32)> {
    let e = 1.0 / 8.0;
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| [x0, y0, x1, y1];
    let quads = |ul: bool, ur: bool, ll: bool, lr: bool| {
        let mut v = Vec::new();
        if ul { v.push(r(0.0, 0.0, 0.5, 0.5)) }
        if ur { v.push(r(0.5, 0.0, 1.0, 0.5)) }
        if ll { v.push(r(0.0, 0.5, 0.5, 1.0)) }
        if lr { v.push(r(0.5, 0.5, 1.0, 1.0)) }
        v
    };
    Some(match c {
        '▀' => (vec![r(0.0, 0.0, 1.0, 0.5)], 1.0),
        '▁'..='█' => {
            let n = (c as u32 - '▁' as u32 + 1) as f32;
            (vec![r(0.0, 1.0 - n * e, 1.0, 1.0)], 1.0)
        }
        '▉'..='▏' => {
            let n = (8 - (c as u32 - '▉' as u32 + 1)) as f32;
            (vec![r(0.0, 0.0, n * e, 1.0)], 1.0)
        }
        '▐' => (vec![r(0.5, 0.0, 1.0, 1.0)], 1.0),
        '░' => (vec![r(0.0, 0.0, 1.0, 1.0)], 0.25),
        '▒' => (vec![r(0.0, 0.0, 1.0, 1.0)], 0.5),
        '▓' => (vec![r(0.0, 0.0, 1.0, 1.0)], 0.75),
        '▔' => (vec![r(0.0, 0.0, 1.0, e)], 1.0),
        '▕' => (vec![r(1.0 - e, 0.0, 1.0, 1.0)], 1.0),
        '▖' => (quads(false, false, true, false), 1.0),
        '▗' => (quads(false, false, false, true), 1.0),
        '▘' => (quads(true, false, false, false), 1.0),
        '▙' => (quads(true, false, true, true), 1.0),
        '▚' => (quads(true, false, false, true), 1.0),
        '▛' => (quads(true, true, true, false), 1.0),
        '▜' => (quads(true, true, false, true), 1.0),
        '▝' => (quads(false, true, false, false), 1.0),
        '▞' => (quads(false, true, true, false), 1.0),
        '▟' => (quads(false, true, true, true), 1.0),
        _ => return None,
    })
}

/// Octant patterns Unicode encodes elsewhere (blank, quadrants, halves,
/// quarter blocks): U+1CD00.. lists the other 230 in increasing order, with
/// bit n set for octant n+1 (two per row, top to bottom).
const OCTANTS_ELSEWHERE: [u8; 26] = [0, 1, 2, 3, 5, 10, 15, 20, 40, 63, 64, 80, 85, 90, 95, 128, 160, 165, 170, 175, 192, 240, 245, 250, 252, 255];

/// Sextants, octants and the eighth blocks, as Ghostty draws them
/// (src/font/sprite/draw/symbols_for_legacy_computing*.zig).
fn legacy_blocks(code: u32) -> Option<Vec<([f32; 4], f32)>> {
    let e = 1.0 / 8.0;
    let solid = |v: Vec<[f32; 4]>| v.into_iter().map(|r| (r, 1.0)).collect::<Vec<_>>();
    // Two columns of `rows` rows; bit n fills cell n (left to right, top down).
    let grid = |bits: u32, rows: u32| {
        let h = 1.0 / rows as f32;
        solid((0..rows * 2).filter(|n| bits & (1 << n) != 0).map(|n| {
            let (col, row) = ((n % 2) as f32, (n / 2) as f32);
            [col * 0.5, row * h, col * 0.5 + 0.5, (row + 1.0) * h]
        }).collect())
    };
    let upper = |k: f32| [0.0, 0.0, 1.0, k * e];
    let lower = |k: f32| [0.0, 1.0 - k * e, 1.0, 1.0];
    let left = |k: f32| [0.0, 0.0, k * e, 1.0];
    let right = |k: f32| [1.0 - k * e, 0.0, 1.0, 1.0];
    let row = |n: f32| [0.0, n * e, 1.0, (n + 1.0) * e];
    Some(match code {
        // Sextants skip the patterns that are blank, a half or full.
        0x1FB00..=0x1FB3B => {
            let i = code - 0x1FB00;
            grid(i + i / 0x14 + 1, 3)
        }
        0x1CD00..=0x1CDE5 => {
            let bits = (0..=255u8).filter(|b| !OCTANTS_ELSEWHERE.contains(b)).nth((code - 0x1CD00) as usize)?;
            grid(u32::from(bits), 4)
        }
        0x1FB70..=0x1FB75 => {
            let n = (code - 0x1FB70 + 1) as f32;
            solid(vec![[n * e, 0.0, (n + 1.0) * e, 1.0]])
        }
        0x1FB76..=0x1FB7B => solid(vec![row((code - 0x1FB76 + 1) as f32)]),
        0x1FB7C => solid(vec![left(1.0), lower(1.0)]),
        0x1FB7D => solid(vec![left(1.0), upper(1.0)]),
        0x1FB7E => solid(vec![right(1.0), upper(1.0)]),
        0x1FB7F => solid(vec![right(1.0), lower(1.0)]),
        0x1FB80 => solid(vec![upper(1.0), lower(1.0)]),
        0x1FB81 => solid(vec![row(0.0), row(2.0), row(4.0), row(7.0)]),
        0x1FB82..=0x1FB86 => solid(vec![upper([2.0, 3.0, 5.0, 6.0, 7.0][(code - 0x1FB82) as usize])]),
        0x1FB87..=0x1FB8B => solid(vec![right([2.0, 3.0, 5.0, 6.0, 7.0][(code - 0x1FB87) as usize])]),
        0x1FB8C => vec![(left(4.0), 0.5)],
        0x1FB8D => vec![(right(4.0), 0.5)],
        0x1FB8E => vec![(upper(4.0), 0.5)],
        0x1FB8F => vec![(lower(4.0), 0.5)],
        0x1FB90 => vec![([0.0, 0.0, 1.0, 1.0], 0.5)],
        // A medium shade with one half solid.
        0x1FB91 => vec![([0.0, 0.5, 1.0, 1.0], 0.5), (upper(4.0), 1.0)],
        0x1FB92 => vec![([0.0, 0.0, 1.0, 0.5], 0.5), (lower(4.0), 1.0)],
        0x1FB94 => vec![([0.0, 0.0, 0.5, 1.0], 0.5), (right(4.0), 1.0)],
        0x1FB97 => solid(vec![[0.0, 0.25, 1.0, 0.5], [0.0, 0.75, 1.0, 1.0]]),
        _ => return None,
    })
}

/// Smooth mosaics (U+1FB3C..1FB67) as Ghostty's 3x4 patterns: `#` filled,
/// `.` empty, `/` and `\` a diagonal edge through that spot.
const MOSAICS: [&str; 44] = [
    "......#..##.", "......#\\.###", "...#..#\\.##.", "...#..##.###",
    "#..#..##.##.", "/###########", "./##########", ".##.########",
    "..#.########", ".##.##.#####", "..../#######", "........#.##",
    "......./####", ".....#./#.##", ".....#.#####", "..#..#.##.##",
    "##\\#########", "#\\.#########", "##.##.######", "#..##.######",
    "##.##.##.###", "...#\\.######", "#########\\##", "#########.\\#",
    "######.##.##", "######.##..#", "###.##.##.##", "##.#........",
    "####/.......", "##.#/.#.....", "#####.#.....", "##.##.#..#..",
    "#######/....", "###########/", "##########/.", "########.##.",
    "########.#..", "#####.##.##.", ".##..#......", "###.\\#......",
    ".##.\\#..#...", "###.##..#...", ".##.##..#..#", "######.\\#...",
];

/// A smooth mosaic's outline in cell fractions: Ghostty walks ten spots around
/// the cell (corners, thirds of the sides, middles of top and bottom) and
/// keeps those the pattern fills, skipping one between two filled neighbors.
fn mosaic(code: u32) -> Option<Vec<(f32, f32)>> {
    let p = MOSAICS.get(code.checked_sub(0x1FB3C)? as usize)?.as_bytes();
    if p.len() != 12 {
        return None;
    }
    let f = |i: usize| p[i] == b'#';
    let (t, u, l, b) = (0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0);
    let spots = [
        (f(0), (0.0, t)),
        (f(3) && (!f(0) || !f(6)), (0.0, u)),
        (f(6) && (!f(3) || !f(9)), (0.0, l)),
        (f(9), (0.0, b)),
        (f(10) && (!f(9) || !f(11)), (0.5, b)),
        (f(11), (1.0, b)),
        (f(8) && (!f(11) || !f(5)), (1.0, l)),
        (f(5) && (!f(8) || !f(2)), (1.0, u)),
        (f(2), (1.0, t)),
        (f(1) && (!f(2) || !f(0)), (0.5, t)),
    ];
    Some(spots.into_iter().filter(|(on, _)| *on).map(|(_, p)| p).collect())
}

/// Filled shapes in cell fractions: smooth mosaics, the triangles from an
/// edge to the center (U+1FB68..1FB6F, the first four inverted) and the
/// corner triangles ◢◣◤◥.
fn polygons(c: char) -> Option<Vec<Vec<(f32, f32)>>> {
    let code = c as u32;
    let edge = |k: u32| -> [(f32, f32); 3] {
        let m = (0.5, 0.5);
        match k {
            0 => [m, (0.0, 0.0), (0.0, 1.0)], // left
            1 => [m, (1.0, 0.0), (0.0, 0.0)], // top
            2 => [m, (1.0, 1.0), (1.0, 0.0)], // right
            _ => [m, (0.0, 1.0), (1.0, 1.0)], // bottom
        }
    };
    Some(match code {
        0x1FB3C..=0x1FB67 => vec![mosaic(code)?],
        // The cell without one edge triangle: the other three.
        0x1FB68..=0x1FB6B => (0..4).filter(|k| *k != code - 0x1FB68).map(|k| edge(k).to_vec()).collect(),
        0x1FB6C..=0x1FB6F => vec![edge(code - 0x1FB6C).to_vec()],
        0x25E2 => vec![vec![(1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]],
        0x25E3 => vec![vec![(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)]],
        0x25E4 => vec![vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]],
        0x25E5 => vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
        _ => return None,
    })
}

fn is_braille(c: char) -> bool {
    ('\u{2800}'..='\u{28FF}').contains(&c)
}

/// 🮕 🮖: a checkerboard four squares wide, with the parity of its top left square.
fn checkerboard(c: char) -> Option<u32> {
    match c as u32 {
        0x1FB95 => Some(0),
        0x1FB96 => Some(1),
        _ => None,
    }
}

/// U+1FB93 is unassigned, a hole in the block; Ghostty draws it empty.
const HOLE: char = '\u{1FB93}';

/// ╱ ╲ ╳: lines from corner to corner.
fn is_diagonal(c: char) -> bool {
    ('\u{2571}'..='\u{2573}').contains(&c)
}

/// ⏺ (Claude Code marks every step with it): few terminal fonts have it, and
/// the fallback may draw nothing.
const RECORD: char = '\u{23FA}';

pub fn is_sprite(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            lines(c).is_some()
                || blocks(c).is_some()
                || polygons(c).is_some()
                || is_braille(c)
                || is_diagonal(c)
                || checkerboard(c).is_some()
                || c == HOLE
                || ('\u{E0B0}'..='\u{E0BF}').contains(&c)
                || c == RECORD
        }
        _ => false,
    }
}

/// Paints `text` (one sprite character) into `cell`, snapped to device pixels.
pub fn paint(text: &str, cell: Bounds<Pixels>, color: Hsla, window: &mut Window) {
    let Some(c) = text.chars().next() else { return };
    let scale = window.scale_factor();
    let snap = |v: f32| (v * scale).round() / scale;
    let (x, y, w, h) = (f32::from(cell.origin.x), f32::from(cell.origin.y), f32::from(cell.size.width), f32::from(cell.size.height));
    let mut rect = |x0: f32, y0: f32, x1: f32, y1: f32, color: Hsla| {
        let (x0, y0, x1, y1) = (snap(x0), snap(y0), snap(x1), snap(y1));
        if x1 > x0 && y1 > y0 {
            window.paint_quad(fill(Bounds::new(point(px(x0), px(y0)), size(px(x1 - x0), px(y1 - y0))), color));
        }
    };

    if c == RECORD {
        let d = snap((w * 0.62).min(h * 0.5));
        let (x0, y0) = (snap(x + (w - d) / 2.0), snap(y + (h - d) / 2.0));
        window.paint_quad(fill(Bounds::new(point(px(x0), px(y0)), size(px(d), px(d))), color).corner_radii(px(d / 2.0)));
        return;
    }
    if ('\u{E0B0}'..='\u{E0BF}').contains(&c) {
        let line = (w / 8.0).round().max(1.0 / scale);
        powerline(c, (x, y, w, h), line, color, window);
        return;
    }
    if let Some(rects) = blocks(c) {
        for ([x0, y0, x1, y1], alpha) in rects {
            let mut color = color;
            color.a *= alpha;
            rect(x + x0 * w, y + y0 * h, x + x1 * w, y + y1 * h, color);
        }
        return;
    }
    if let Some(parity) = checkerboard(c) {
        // Ghostty's grid: square edges on whole device pixels, rows as tall as the columns are wide.
        let (cw, ch) = ((w * scale).round() as u32, (h * scale).round() as u32);
        let rows = ((4.0 * h / w).round() as u32).max(1);
        for gx in 0..4 {
            for gy in (0..rows).filter(|gy| (gx + gy) % 2 == parity) {
                let (x0, x1) = ((cw * gx / 4) as f32 / scale, (cw * (gx + 1) / 4) as f32 / scale);
                let (y0, y1) = ((ch * gy / rows) as f32 / scale, (ch * (gy + 1) / rows) as f32 / scale);
                rect(x + x0, y + y0, x + x1, y + y1, color);
            }
        }
        return;
    }
    if let Some(shapes) = polygons(c) {
        // Edges on the cell's sides reach a device pixel out, so neighbors meet without a seam.
        let ext = 1.0 / scale;
        let out = |v: f32, size: f32| if v <= 0.0 { -ext } else if v >= 1.0 { size + ext } else { v * size };
        for shape in shapes {
            let pts: Vec<_> = shape.iter().map(|&(px_, py_)| point(px(x + out(px_, w)), px(y + out(py_, h)))).collect();
            let mut path = PathBuilder::fill();
            path.add_polygon(&pts, true);
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        }
        return;
    }
    if is_braille(c) {
        braille(c, (x, y, w, h), scale, color, window);
        return;
    }
    if is_diagonal(c) {
        let light = (w / 8.0).round().max(1.0 / scale);
        let p = |px_: f32, py_: f32| point(px(x + px_ * w), px(y + py_ * h));
        let mut path = PathBuilder::stroke(px(light));
        if c != '\u{2572}' {
            path.add_polygon(&[p(1.0, 0.0), p(0.0, 1.0)], false);
        }
        if c != '\u{2571}' {
            path.add_polygon(&[p(0.0, 0.0), p(1.0, 1.0)], false);
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
        return;
    }
    let Some(dirs) = lines(c) else { return };
    let light = (w / 8.0).round().max(1.0 / scale);
    let thick = |k: u8| if k == 2 { light * 2.0 } else { light };
    let (cx, cy) = (snap(x + w / 2.0), snap(y + h / 2.0));
    // Where a line along an arm starts, measured from the center toward the
    // arm: `before` and `after` are the crossing arms on this line's side and
    // the other side. Negative reaches past the center so joins close.
    let cover = |k: u8| if k == 3 { light * 1.5 } else { thick(k) / 2.0 };
    let start = |before: u8, after: u8, double: bool| -> f32 {
        match (before, after) {
            // A double line stops at a double crossing on its side (inner corner).
            (3, _) if double => light / 2.0,
            (0, 0) => 0.0,
            (0, k) | (k, _) => -cover(k),
        }
    };
    // Arms: (direction, sign along the axis, horizontal?).
    for (i, sign, horizontal) in [(0usize, -1.0f32, false), (1, 1.0, true), (2, 1.0, false), (3, -1.0, true)] {
        let k = dirs[i];
        if k == 0 {
            continue;
        }
        // The crossing arms on each side of this arm (up/down for a horizontal arm).
        let (side_a, side_b) = if horizontal { (dirs[0], dirs[2]) } else { (dirs[3], dirs[1]) };
        let lines: Vec<(f32, f32, u8, u8)> = if k == 3 {
            vec![(-light, light, side_a, side_b), (light, light, side_b, side_a)]
        } else {
            let (a, b) = if side_a >= side_b { (side_a, side_b) } else { (side_b, side_a) };
            vec![(0.0, thick(k), a, b)]
        };
        for (off, t, before, after) in lines {
            let near = start(before, after, k == 3);
            let (c, edge) = if horizontal { (cx, if sign > 0.0 { x + w } else { x }) } else { (cy, if sign > 0.0 { y + h } else { y }) };
            let from = c + sign * near;
            let (a0, a1) = if sign > 0.0 { (from, edge) } else { (edge, from) };
            if horizontal {
                rect(a0, cy + off - t / 2.0, a1, cy + off + t / 2.0, color);
            } else {
                rect(cx + off - t / 2.0, a0, cx + off + t / 2.0, a1, color);
            }
        }
    }
}

/// Braille (U+2800..28FF): square dots on a 2x4 grid, laid out in whole
/// device pixels as Ghostty does (src/font/sprite/draw/braille.zig), so every
/// dot is the same size.
fn braille(c: char, (x, y, w, h): (f32, f32, f32, f32), scale: f32, color: Hsla, window: &mut Window) {
    let (width, height) = ((w * scale).round() as i32, (h * scale).round() as i32);
    let mut dot = (width / 4).min(height / 8);
    let (mut x_spacing, mut y_spacing) = (width / 4, height / 8);
    let (mut x_margin, mut y_margin) = (x_spacing / 2, y_spacing / 2);
    let mut x_left = width - 2 * x_margin - x_spacing - 2 * dot;
    let mut y_left = height - 2 * y_margin - 3 * y_spacing - 4 * dot;
    if x_left >= 2 && y_left >= 4 && dot == 0 {
        dot += 1;
        x_left -= 2;
        y_left -= 4;
    }
    if x_left >= 2 && x_margin == 0 {
        x_margin = 1;
        x_left -= 2;
    }
    if y_left >= 2 && y_margin == 0 {
        y_margin = 1;
        y_left -= 2;
    }
    if x_left >= 1 {
        x_spacing += 1;
        x_left -= 1;
    }
    if y_left >= 3 {
        y_spacing += 1;
        y_left -= 3;
    }
    if x_left >= 2 {
        x_margin += 1;
        x_left -= 2;
    }
    if y_left >= 2 {
        y_margin += 1;
        y_left -= 2;
    }
    if x_left >= 2 && y_left >= 4 {
        dot += 1;
    }
    let cols = [x_margin, x_margin + dot + x_spacing];
    let rows: Vec<i32> = (0..4).map(|r| y_margin + r * (dot + y_spacing)).collect();
    // Unicode's dot order: 1-3 down the left, 4-6 down the right, then 7 and 8.
    const DOTS: [(usize, usize); 8] = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)];
    let bits = c as u32 - 0x2800;
    let (x0, y0) = ((x * scale).round(), (y * scale).round());
    for (n, (col, row)) in DOTS.iter().enumerate() {
        if bits & (1 << n) != 0 {
            let at = point(px((x0 + cols[*col] as f32) / scale), px((y0 + rows[*row] as f32) / scale));
            window.paint_quad(fill(Bounds::new(at, size(px(dot as f32 / scale), px(dot as f32 / scale))), color));
        }
    }
}

/// Nerd Font powerline separators (U+E0B0..E0BF): arrows, half circles and
/// slants, drawn to fill the cell exactly as Ghostty does. Fonts that have
/// them rarely match the cell, and fonts that don't fall back to whatever
/// font has those codes (on Linux, sometimes digits).
fn powerline(c: char, (x, y, w, h): (f32, f32, f32, f32), line: f32, color: Hsla, window: &mut Window) {
    let solid = (c as u32).is_multiple_of(2);
    // A solid shape reaches a device pixel past its cell's sides, so its flat
    // edge meets the neighbor's color without an anti-aliased seam.
    let ext = if solid { 1.0 / window.scale_factor() } else { 0.0 };
    let p = |px_: f32, py_: f32| point(px(x + px_ * w + (px_ - 0.5) * 2.0 * ext), px(y + py_ * h));
    let mut path = if solid { PathBuilder::fill() } else { PathBuilder::stroke(px(line)) };
    // Half an ellipse filling the cell: four quarter arcs as cubics.
    const K: f32 = 0.552_284_8;
    match c {
        '\u{E0B0}' | '\u{E0B1}' => path.add_polygon(&[p(0.0, 0.0), p(1.0, 0.5), p(0.0, 1.0)], solid),
        '\u{E0B2}' | '\u{E0B3}' => path.add_polygon(&[p(1.0, 0.0), p(0.0, 0.5), p(1.0, 1.0)], solid),
        '\u{E0B4}' | '\u{E0B5}' => {
            path.move_to(p(0.0, 0.0));
            path.cubic_bezier_to(p(1.0, 0.5), p(K, 0.0), p(1.0, 0.5 - K / 2.0));
            path.cubic_bezier_to(p(0.0, 1.0), p(1.0, 0.5 + K / 2.0), p(K, 1.0));
            if solid {
                path.close();
            }
        }
        '\u{E0B6}' | '\u{E0B7}' => {
            path.move_to(p(1.0, 0.0));
            path.cubic_bezier_to(p(0.0, 0.5), p(1.0 - K, 0.0), p(0.0, 0.5 - K / 2.0));
            path.cubic_bezier_to(p(1.0, 1.0), p(0.0, 0.5 + K / 2.0), p(1.0 - K, 1.0));
            if solid {
                path.close();
            }
        }
        '\u{E0B8}' => path.add_polygon(&[p(0.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)], true),
        '\u{E0BA}' => path.add_polygon(&[p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)], true),
        '\u{E0BC}' => path.add_polygon(&[p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)], true),
        '\u{E0BE}' => path.add_polygon(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)], true),
        '\u{E0B9}' | '\u{E0BF}' => path.add_polygon(&[p(0.0, 0.0), p(1.0, 1.0)], false),
        _ => path.add_polygon(&[p(0.0, 1.0), p(1.0, 0.0)], false), // E0BB, E0BD
    }
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_cover_their_ranges() {
        assert_eq!(lines('─'), Some([0, 1, 0, 1]));
        assert_eq!(lines('┼'), Some([1, 1, 1, 1]));
        assert_eq!(lines('╋'), Some([2, 2, 2, 2]));
        assert_eq!(lines('┌'), Some([0, 1, 1, 0]));
        assert_eq!(lines('┘'), Some([1, 0, 0, 1]));
        assert_eq!(lines('╏'), Some([2, 0, 2, 0]));
        assert_eq!(lines('╔'), Some([0, 3, 3, 0]));
        assert_eq!(lines('╬'), Some([3, 3, 3, 3]));
        assert_eq!(lines('╭'), Some([0, 1, 1, 0]));
        assert_eq!(lines('╰'), Some([1, 1, 0, 0]));
        assert_eq!(lines('╱'), None);
        assert_eq!(lines('╿'), Some([2, 0, 1, 0]));
        for c in ('\u{2500}'..='\u{2570}').chain('\u{2574}'..='\u{257F}') {
            assert!(lines(c).is_some(), "{c} has no entry");
        }
        for c in '\u{2580}'..='\u{259F}' {
            assert!(blocks(c).is_some(), "{c} has no entry");
        }
    }

    #[test]
    fn legacy_computing_mosaics_are_sprites() {
        let ch = |code: u32| char::from_u32(code).unwrap().to_string();
        // Every sextant, octant and smooth mosaic, braille, the diagonals.
        for code in (0x1FB00..=0x1FB6F).chain(0x1CD00..=0x1CDE5).chain(0x1FB70..=0x1FB97).chain(0x2800..=0x28FF).chain(0x2571..=0x2573) {
            assert!(is_sprite(&ch(code)), "U+{code:X} is not drawn");
        }
        assert_eq!((checkerboard('\u{1FB95}'), checkerboard('\u{1FB96}')), (Some(0), Some(1)));
        // 🬀 is the top left sextant; 🬻 all but the top left.
        assert_eq!(legacy_blocks(0x1FB00).unwrap().len(), 1);
        assert_eq!(legacy_blocks(0x1FB3B).unwrap().len(), 5);
        // U+1CD00 is octant 3 alone; U+1CDE5 all but octant 1.
        assert_eq!(legacy_blocks(0x1CD00).unwrap(), vec![([0.0, 0.25, 0.5, 0.5], 1.0)]);
        assert_eq!(legacy_blocks(0x1CDE5).unwrap().len(), 7);
        assert!(legacy_blocks(0x1CDE6).is_none());
        // 🬼: a triangle in the lower left.
        assert_eq!(mosaic(0x1FB3C).unwrap(), vec![(0.0, 2.0 / 3.0), (0.0, 1.0), (0.5, 1.0)]);
    }
}
