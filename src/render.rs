//! ANSI terminal rendering. No GPU, no windowing — the whole world is drawn
//! with escape codes so it runs in any terminal.

use crate::model::*;

const RESET: &str = "\x1b[0m";

fn paint(s: &str, code: &str) -> String {
    format!("\x1b[{}m{}{}", code, s, RESET)
}

/// SGR colour for a town's consumer price (green = cheap, magenta = its
/// weight in gold).
fn price_code(p: f32) -> &'static str {
    if p < 16.0 {
        "1;92" // bright green
    } else if p < 45.0 {
        "1;93" // yellow
    } else if p < 110.0 {
        "1;91" // red
    } else {
        "1;95" // bright magenta
    }
}

/// Single-letter map glyph for each town (index order matches World::towns).
const GLYPH: [char; 10] = ['L', 'U', 'B', 'A', 'S', 'H', 'V', 'T', 'Z', 'K'];

pub fn clear() -> &'static str {
    "\x1b[2J\x1b[H"
}

pub fn frame(w: &World) -> String {
    let mut out = String::new();

    // Header.
    out.push_str(&paint(
        "  WHITE GOLD — A Chronicle of the Salt Roads",
        "1;97",
    ));
    out.push_str(&format!(
        "        {}   (seed {})\n",
        paint(&w.date_label(), "1;96"),
        w.seed
    ));

    // Active world conditions.
    let mut conds: Vec<String> = Vec::new();
    if w.drought > 0 {
        conds.push(paint("DROUGHT", "1;33"));
    }
    if let Some((a, b, t)) = w.war {
        conds.push(paint(&format!("WAR {}~{} ({})", a, b, t), "1;31"));
    }
    if w.towns.iter().any(|t| t.cornered > 0) {
        conds.push(paint("MONOPOLY", "1;35"));
    }
    let cond_line = if conds.is_empty() {
        paint("fair winds", "2;37")
    } else {
        conds.join("  ")
    };
    out.push_str(&format!("  {}\n", cond_line));

    // --- Map ---------------------------------------------------------------
    let mut cells: Vec<(char, &'static str)> = Vec::with_capacity(MAP_W * MAP_H);
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let g = match w.terrain[y * MAP_W + x] {
                Terrain::Sea => ('~', "34"),
                Terrain::Plain => ('.', "2;32"),
                Terrain::Mountain => ('^', "2;37"),
                Terrain::Desert => (':', "2;33"),
            };
            cells.push(g);
        }
    }
    // Overlay caravans.
    for c in &w.caravans {
        let (x, y) = (c.x.round() as i32, c.y.round() as i32);
        if x >= 0 && y >= 0 && (x as usize) < MAP_W && (y as usize) < MAP_H {
            let g = match c.kind {
                CaravanKind::Merchant => ('+', "1;97"),
                CaravanKind::Smuggler => ('!', "1;35"),
            };
            cells[y as usize * MAP_W + x as usize] = g;
        }
    }
    // Overlay towns last so they're always visible.
    for (i, t) in w.towns.iter().enumerate() {
        let (x, y) = (t.x as usize, t.y as usize);
        if x < MAP_W && y < MAP_H {
            cells[y * MAP_W + x] = (GLYPH[i], price_code(t.consumer_price()));
        }
    }

    out.push_str("  +");
    out.push_str(&"-".repeat(MAP_W));
    out.push_str("+\n");
    for y in 0..MAP_H {
        out.push_str("  |");
        // Build the row, coalescing each cell's colour.
        for x in 0..MAP_W {
            let (ch, code) = cells[y * MAP_W + x];
            let mut buf = [0u8; 4];
            out.push_str(&paint(ch.encode_utf8(&mut buf), code));
        }
        out.push_str("|\n");
    }
    out.push_str("  +");
    out.push_str(&"-".repeat(MAP_W));
    out.push_str("+\n");

    // --- Town ledger (two columns) -----------------------------------------
    let half = (w.towns.len() + 1) / 2;
    for row in 0..half {
        out.push_str("  ");
        out.push_str(&town_cell(w, row));
        let r = row + half;
        if r < w.towns.len() {
            out.push_str("  ");
            out.push_str(&town_cell(w, r));
        }
        out.push('\n');
    }

    // --- Chronicle ---------------------------------------------------------
    out.push_str(&paint("\n  The Chronicle\n", "1;97"));
    let start = w.chronicle.len().saturating_sub(8);
    for line in &w.chronicle[start..] {
        out.push_str(&format!("  {}\n", paint(line, "37")));
    }
    if w.chronicle.is_empty() {
        out.push_str(&paint("  (the world is quiet... for now)\n", "2;37"));
    }

    out
}

fn town_cell(w: &World, i: usize) -> String {
    let t = &w.towns[i];
    let cp = t.consumer_price();
    let glyph = paint(&GLYPH[i].to_string(), price_code(cp));
    let name = format!("{:<9}", t.name);
    let price = paint(&format!("{:>4.0}d", cp), price_code(cp));
    let tax = if t.tax > 0.0 {
        paint(&format!("g{:>3.0}%", t.tax * 100.0), if t.tax > 0.5 { "1;31" } else { "33" })
    } else {
        paint("     ", "2;37")
    };
    let works = paint(t.works.tag(), "36");

    // Discontent meter (fills toward a revolt).
    let filled = ((t.discontent / REVOLT_AT) * 6.0).round().clamp(0.0, 6.0) as usize;
    let meter_code = if filled >= 5 {
        "1;31"
    } else if filled >= 2 {
        "33"
    } else {
        "2;32"
    };
    let meter = paint(
        &format!("[{}{}]", "#".repeat(filled), "-".repeat(6 - filled)),
        meter_code,
    );

    format!("{} {} {} {} {} {}", glyph, name, price, tax, works, meter)
}
